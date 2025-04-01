use embedded_io_async::Read;
use impls::FakeSerial;
use postcard_rpc::Key;
use postcard_schema::Schema;
use poststation_sdk::connect;
use r503::{
    auto::{AutoEnroll, AutoEnrollConfig, AutoIdentify, AutoIdentifyConfig}, constants::{AutoIdentCount, CharBufferId, IdentifySafety, IndexTableIdx}, LoadCharRequest, R503
};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::{Read as _, Write}, net::SocketAddr, num::ParseIntError, time::Duration};
use tokio::{
    select,
    time::{sleep, timeout},
};

const TEMPLATE_KEY: Key = Key::for_path::<TemplateExport>("template export");

#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct SingleTemplate {
    idx: u16,
    data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Schema)]
pub struct TemplateExport {
    templates: Vec<SingleTemplate>,
}

pub mod impls;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tokio::task::spawn(inner_main()).await??;
    Ok(())
}

async fn inner_main() -> anyhow::Result<()> {
    let server: SocketAddr = "127.0.0.1:51837".parse().unwrap();
    let client = connect(server).await.unwrap();
    let serial = 0xE462B044CB202439u64;

    let mut serial = FakeSerial::new(&client, serial).await.unwrap();
    serial.set_baudrate(57_600).await.unwrap();

    let r5 = R503::new_with_address(0xFFFFFFFF);
    let rand = r5.get_rand_code(&mut serial).await.unwrap();
    println!("Rand said: {rand:08X}");
    let r5 = &r5;
    let serial = &mut serial;

    'repl: loop {
        print!("> ");
        let _ = std::io::stdout().flush();

        let line = read_line().await;

        // Drain any extra bytes
        loop {
            let mut buf = [0u8; 64];
            match timeout(Duration::from_millis(100), serial.read(&mut buf)).await {
                Ok(_) => {}
                Err(_) => break,
            }
        }

        let tline = line.trim();
        let words = tline.split_whitespace().collect::<Vec<_>>();
        let res = match words.as_slice() {
            ["empty"] => r5.empty(serial).await,
            ["idx", "table", "read"] => read_idx_table(r5, serial).await,
            ["auto", "enroll"] => auto_enroll(r5, serial).await,
            ["auto", "enroll", "loop"] => {
                let fut = async {
                    while auto_enroll(r5, serial).await.is_ok() {
                        println!("REMOVE FINGER!");
                        sleep(Duration::from_secs(3)).await;
                    }
                };
                // run until user presses enter
                select! {
                    _ = fut => {}
                    _ = read_line() => {}
                }
                Ok(())
            }
            ["auto", "identify", grade, start, end, count] => {
                let Some(grade) =
                    hex_or_dec(grade).and_then(|g: u8| IdentifySafety::try_from(g).ok())
                else {
                    println!("Bad grade");
                    continue 'repl;
                };
                let Some(start_pos) = hex_or_dec(start) else {
                    println!("Bad start");
                    continue 'repl;
                };
                let Some(steps_or_end) = hex_or_dec(end) else {
                    println!("Bad end");
                    continue 'repl;
                };
                let Some(err_count) = hex_or_dec::<u8>(count).map(Into::into) else {
                    println!("Bad start");
                    continue 'repl;
                };
                let cfg = AutoIdentifyConfig {
                    grade,
                    start_pos,
                    steps_or_end,
                    return_status: true,
                    err_count,
                };
                auto_identify(r5, serial, cfg).await
            }
            ["auto", "identify", grade, start, end, count, "loop"] => {
                let Some(grade) =
                    hex_or_dec(grade).and_then(|g: u8| IdentifySafety::try_from(g).ok())
                else {
                    println!("Bad grade");
                    continue 'repl;
                };
                let Some(start_pos) = hex_or_dec(start) else {
                    println!("Bad start");
                    continue 'repl;
                };
                let Some(steps_or_end) = hex_or_dec(end) else {
                    println!("Bad end");
                    continue 'repl;
                };
                let Some(err_count) = hex_or_dec::<u8>(count).map(Into::into) else {
                    println!("Bad start");
                    continue 'repl;
                };
                let cfg = AutoIdentifyConfig {
                    grade,
                    start_pos,
                    steps_or_end,
                    return_status: true,
                    err_count,
                };
                let fut = async {
                    while let Ok(()) = auto_identify(r5, serial, cfg.clone()).await {
                        println!("REMOVE FINGER!");
                        sleep(Duration::from_secs(3)).await;
                    }
                };
                // run until user presses enter
                select! {
                    _ = fut => {}
                    _ = read_line() => {}
                }
                Ok(())
            }
            ["dump", "templates", path] => {
                dump_templates(r5, serial, path).await.unwrap();
                Ok(())
            },
            ["debugload", "templates", path] => {
                debugload_templates(path).unwrap();
                Ok(())
            }
            other => {
                println!("Error, didn't understand: {other:?}");
                Ok(())
            }
        };
        match res {
            Ok(()) => println!("Success"),
            Err(e) => println!("Error: {e:?}"),
        }
    }
}

fn debugload_templates(path: &str) -> Result<(), r503::Error<FakeSerial>> {
    let mut buf = vec![];
    let mut f = File::open(path).unwrap();
    f.read_to_end(&mut buf).unwrap();
    let (now, later) = postcard::take_from_bytes::<Key>(&buf).unwrap();
    if now != TEMPLATE_KEY {
        panic!("Wrong header!");
    }
    let templates = postcard::from_bytes::<TemplateExport>(later).unwrap();
    for temp in templates.templates {
        println!("# Template {}", temp.idx);
        println!();
        for ch in temp.data.chunks(16) {
            for b in ch {
                print!("{b:02X} ");
            }
            println!();
        }
        println!();
    }
    Ok(())
}

async fn dump_templates<'a>(r5: &R503, serial: &mut FakeSerial, path: &str) -> Result<(), r503::Error<FakeSerial>> {
    let mut templates: Vec<u16> = vec![];
    for i in 0..4 {
        let idx = IndexTableIdx::try_from(i).unwrap();
        let val = r5.read_idx_table(serial, idx).await?;
        for (j, mut by) in val.into_iter().enumerate() {
            for k in 0..8 {
                if by & 0x01 != 0 {
                    templates.push((i as u16 * 256u16) + (j * 8) as u16 + k);
                }
                by >>= 1;
            }
        }
    }

    let mut out = vec![];
    let mut buf = vec![0u8; 512];
    for template in templates {
        println!("Loading template {template}");
        // clear data
        buf.iter_mut().for_each(|b| *b = 0);
        r5.load_char(serial, LoadCharRequest { char_buffer: CharBufferId::One, model_id: template }).await?;
        r5.upload_template(serial, CharBufferId::One).await?;
        let used = r5.stream_image(serial, &mut buf).await?;
        if used != 512 {
            return Err(r503::Error::IncorrectData);
        }
        out.push(SingleTemplate { idx: template, data: buf.clone() });
    }

    let all_templates = TemplateExport { templates: out };
    let mut payload = postcard::to_stdvec(&TEMPLATE_KEY).unwrap();
    let body = postcard::to_stdvec(&all_templates).unwrap();
    payload.extend_from_slice(&body);
    let mut file = File::create(path).unwrap();
    file.write_all(&payload).unwrap();
    file.flush().unwrap();
    println!("Wrote file to '{path}'");
    Ok(())
}

async fn read_idx_table(r5: &R503, serial: &mut FakeSerial) -> Result<(), r503::Error<FakeSerial>> {
    for i in 0..4 {
        println!("# {i}");
        let idx = IndexTableIdx::try_from(i).unwrap();
        let val = r5.read_idx_table(serial, idx).await?;
        for ch in val.chunks(4) {
            for by in ch {
                let mut by = *by;
                for _ in 0..8 {
                    if by & 0x01 != 0 {
                        print!("F");
                    } else {
                        print!("_");
                    }
                    by >>= 1;
                }
                print!(" ");
            }
            println!();
        }
        println!();
    }
    Ok(())
}

async fn auto_identify(
    r5: &R503,
    serial: &mut FakeSerial,
    cfg: AutoIdentifyConfig,
) -> Result<(), r503::Error<FakeSerial>> {
    let mut identify = AutoIdentify::new(r5.address(), serial);
    let err_count = cfg.err_count;
    identify.start(cfg).await?;
    println!("START AUTO Identify");

    let times = match err_count {
        AutoIdentCount::Infinite => usize::MAX,
        AutoIdentCount::TimesWithTimeout(n) => n as usize,
    };

    for _ in 0..times {
        println!("Collect Image...");
        identify.wait_collect_image().await?;
        println!("Generate Template...");
        identify.wait_generate_feature().await?;
        println!("Searching...");
        let res = identify.wait_search().await;
        match res {
            Ok(resp) => {
                println!("Match! ID: {} Score: {}", resp.model_id, resp.score);
                break;
            }
            Err(e) => println!("ERR: {e:?}"),
        }
    }

    Ok(())
}

async fn auto_enroll(r5: &R503, serial: &mut FakeSerial) -> Result<(), r503::Error<FakeSerial>> {
    let mut enroll = AutoEnroll::new(r5.address(), serial);
    println!("START AUTO ENROLL");
    enroll.start(AutoEnrollConfig::default()).await?;
    println!("wait_collect_image1...");
    enroll.wait_collect_image1().await?;
    println!("wait_generate_feature1...");
    enroll.wait_generate_feature1().await?;
    println!("wait_collect_image2...");
    enroll.wait_collect_image2().await?;
    println!("wait_generate_feature2...");
    enroll.wait_generate_feature2().await?;
    println!("wait_collect_image3...");
    enroll.wait_collect_image3().await?;
    println!("wait_generate_feature3...");
    enroll.wait_generate_feature3().await?;
    println!("wait_collect_image4...");
    enroll.wait_collect_image4().await?;
    println!("wait_generate_feature4...");
    enroll.wait_generate_feature4().await?;
    println!("wait_collect_image5...");
    enroll.wait_collect_image5().await?;
    println!("wait_generate_feature5...");
    enroll.wait_generate_feature5().await?;
    println!("wait_collect_image6...");
    enroll.wait_collect_image6().await?;
    println!("wait_generate_feature6...");
    enroll.wait_generate_feature6().await?;
    println!("wait_repeatfingerprint...");
    enroll.wait_repeatfingerprint().await?;
    println!("wait_merge_feature...");
    enroll.wait_merge_feature().await?;
    println!("wait_storage_template...");
    let id = enroll.wait_storage_template().await?;
    println!("Stored to id {id}");
    Ok(())
}

/////////////////////////////////////////////////
// REPL helpers
/////////////////////////////////////////////////

async fn read_line() -> String {
    tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        line
    })
    .await
    .unwrap()
}

pub trait FromStrRadix: Sized {
    fn from_str_radix_gen(src: &str, radix: u32) -> Result<Self, ParseIntError>;
}

macro_rules! fsr_impl {
    ($($typ:ty),+) => {
        $(
            impl FromStrRadix for $typ {
                fn from_str_radix_gen(src: &str, radix: u32) -> Result<Self, ParseIntError> {
                    Self::from_str_radix(src, radix)
                }
            }
        )+
    };
}

fsr_impl!(u8, u16, u32, u64, u128, usize);

pub fn hex_or_dec<T: FromStrRadix>(mut s: &str) -> Option<T> {
    let radix;
    if s.starts_with("0x") {
        radix = 16;
        s = s.trim_start_matches("0x");
    } else if s.ends_with("h") {
        radix = 16;
        s = s.trim_end_matches("h");
    } else {
        radix = 10;
    }
    T::from_str_radix_gen(s, radix).ok()
}

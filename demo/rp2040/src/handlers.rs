use core::sync::atomic::{compiler_fence, Ordering};

use embassy_time::{Instant, Timer};
use postcard_rpc::{header::VarHeader, server::Sender};
use uartbridge_icd::{SleepEndpoint, SleepMillis, SleptMillis, UartFrame};

use crate::app::{AppTx, Context, TaskContext};

/// This is an example of a BLOCKING handler.
pub fn unique_id(context: &mut Context, _header: VarHeader, _arg: ()) -> u64 {
    context.unique_id
}

/// Also a BLOCKING handler
pub fn picoboot_reset(_context: &mut Context, _header: VarHeader, _arg: ()) {
    embassy_rp::rom_data::reset_to_usb_boot(0, 0);
    loop {
        // Wait for reset...
        compiler_fence(Ordering::SeqCst);
    }
}

pub async fn set_baudrate_handler(context: &mut Context, _header: VarHeader, arg: u32) {
    let mut guard = context.serial.lock().await;
    guard.set_baudrate(arg);
    context.baudrate = arg;
}

pub async fn get_baudrate_handler(context: &mut Context, _header: VarHeader, _arg: ()) -> u32 {
    context.baudrate
}

pub async fn send_uart_handler(
    context: &mut Context,
    _header: VarHeader,
    arg: UartFrame<'_>,
    _sender: &Sender<AppTx>,
) {
    use embedded_io_async::Write;

    for chunk in arg.data.chunks(32) {
        let mut guard = context.serial.lock().await;
        let _ = guard.write_all(chunk).await;
    }
}

/// This is a SPAWN handler
///
/// The pool size of three means we can have up to three of these requests "in flight"
/// at the same time. We will return an error if a fourth is requested at the same time
#[embassy_executor::task(pool_size = 3)]
pub async fn sleep_handler(
    _context: TaskContext,
    header: VarHeader,
    arg: SleepMillis,
    sender: Sender<AppTx>,
) {
    // We can send string logs, using the sender
    let _ = sender.log_str("Starting sleep...").await;
    let start = Instant::now();
    Timer::after_millis(arg.millis.into()).await;
    let _ = sender.log_str("Finished sleep").await;
    // Async handlers have to manually reply, as embassy doesn't support returning by value
    let _ = sender
        .reply::<SleepEndpoint>(
            header.seq_no,
            &SleptMillis {
                millis: start.elapsed().as_millis() as u16,
            },
        )
        .await;
}

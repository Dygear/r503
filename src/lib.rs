#![cfg_attr(not(any(feature = "std", test)), no_std)]

use core::fmt::Debug;

use constants::{AuraControlPayload, CharBufferId, Commands, ConfirmationCode, IndexTableIdx, PackageIdentifier};
use embedded_io_async::{ErrorType, Read, ReadExactError, Write};
use wire_traits::{FromWire, ToWire};

pub mod auto;
pub mod constants;
pub mod wire_traits;

//////////////////////////////////////////////////////////////////////////////
// Error
//////////////////////////////////////////////////////////////////////////////

pub enum Error<S>
where
    S: ErrorType,
{
    Wire(S::Error),
    IncorrectData,
    EndOfFile,
    BadConfirmation(ConfirmationCode),
    BadChecksum,
}

impl<S> Debug for Error<S>
where
    S: ErrorType,
    S::Error: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Wire(w) => {
                f.write_str("Error::Wire(")?;
                f.write_fmt(format_args!("{w:?}"))?;
                f.write_str(")")?;
                Ok(())
            }
            Error::IncorrectData => f.write_str("Error::IncorrectData"),
            Error::EndOfFile => f.write_str("Error::EndOfFile"),
            Error::BadConfirmation(c) => {
                f.write_str("Error::BadConfirmation(")?;
                f.write_fmt(format_args!("{c:?}"))?;
                f.write_str(")")?;
                Ok(())
            }
            Error::BadChecksum => f.write_str("Error::BadChecksum"),
        }
    }
}

//////////////////////////////////////////////////////////////////////////////
// Command Packet Type
//////////////////////////////////////////////////////////////////////////////

pub struct Command<T: ToWire> {
    address: u32,
    instruction: Commands,
    body: T,
}

impl<T: ToWire> Command<T> {
    pub async fn to_wire<S>(&self, serial: &mut S) -> Result<(), Error<S>>
    where
        S: Write + ErrorType,
    {
        // Header
        0xEF01u16.to_wire(serial, None).await?;
        // Adder
        self.address.to_wire(serial, None).await?;

        // CRC starts here!
        let mut crc = Checksum::new();

        // Package Identifier
        PackageIdentifier::CommandPacket
            .to_wire(serial, Some(&mut crc))
            .await?;

        // length
        let blen = self.body.size_on_wire();
        // command + CRC
        ((3 + blen) as u16).to_wire(serial, Some(&mut crc)).await?;

        // command
        self.instruction.to_wire(serial, Some(&mut crc)).await?;

        // body (optional)
        self.body.to_wire(serial, Some(&mut crc)).await?;

        // CRC
        crc.finalize().to_wire(serial, None).await?;

        Ok(())
    }
}

//////////////////////////////////////////////////////////////////////////////
// Acknowledge Packet Type
//////////////////////////////////////////////////////////////////////////////

pub struct Response<T> {
    address: u32,
    ident: u8,
    confirmation: ConfirmationCode,
    body: T,
}

impl<T> Response<T> {
    pub async fn from_wire<S: ErrorType + Read>(serial: &mut S) -> Result<Self, Error<S>>
    where
        T: FromWire,
    {
        // Do we have the right header?
        let hdr = u16::from_wire(serial, None).await?;
        if hdr != 0xEF01 {
            return Err(Error::IncorrectData);
        }

        let address = u32::from_wire(serial, None).await?;

        // The remaining bits are checksum relevant!
        let mut cksm = Checksum::new();
        let ident = u8::from_wire(serial, Some(&mut cksm)).await?;
        // TODO: check len?
        let _len = u16::from_wire(serial, Some(&mut cksm)).await?;
        let confirmation = ConfirmationCode::from_wire(serial, Some(&mut cksm)).await?;
        let body = T::from_wire(serial, Some(&mut cksm)).await?;

        let calc_cksm = cksm.finalize();
        let rept_cksm = u16::from_wire(serial, None).await?;

        if calc_cksm != rept_cksm {
            return Err(Error::BadChecksum);
        }
        Ok(Self {
            address,
            ident,
            confirmation,
            body,
        })
    }
}

//////////////////////////////////////////////////////////////////////////////
// Checksum Handler
//////////////////////////////////////////////////////////////////////////////

pub struct Checksum {
    state: u16,
}

impl Checksum {
    pub fn new() -> Self {
        Self { state: 0 }
    }

    pub fn update(&mut self, data: &[u8]) {
        data.iter().copied().for_each(|b| {
            self.state = self.state.wrapping_add(b.into());
        });
    }

    pub fn finalize(self) -> u16 {
        self.state
    }
}

impl Default for Checksum {
    fn default() -> Self {
        Self::new()
    }
}

//////////////////////////////////////////////////////////////////////////////
// R503
//////////////////////////////////////////////////////////////////////////////

pub struct R503 {
    address: u32,
}

impl R503 {
    pub fn new_with_address(addr: u32) -> Self {
        Self { address: addr }
    }

    pub fn address(&self) -> u32 {
        self.address
    }

    pub async fn stream_image<S: Read + ErrorType>(
        &self,
        serial: &mut S,
        out_buf: &mut [u8],
    ) -> Result<usize, Error<S>> {
        let mut more = true;
        let ttl_len = out_buf.len();
        let mut window = out_buf;
        while more {
            // Do we have the right header?
            let hdr = u16::from_wire(serial, None).await?;
            if hdr != 0xEF01 {
                return Err(Error::IncorrectData);
            }

            let address = u32::from_wire(serial, None).await?;
            if address != self.address {
                return Err(Error::IncorrectData);
            }

            // The remaining bits are checksum relevant!
            let mut cksm = Checksum::new();
            let ident = u8::from_wire(serial, Some(&mut cksm)).await?;

            match ident {
                0x02 => {
                    // "Have following packet"
                }
                0x08 => {
                    // "end packet"
                    more = false;
                }
                _ => return Err(Error::IncorrectData),
            }

            let len = u16::from_wire(serial, Some(&mut cksm)).await?;

            if len < 2 {
                return Err(Error::IncorrectData);
            }
            let len_img = (len - 2) as usize;
            if window.len() < len_img {
                // TODO better error
                return Err(Error::IncorrectData);
            }
            let (now, later) = window.split_at_mut(len_img);
            window = later;
            match serial.read_exact(now).await {
                Ok(()) => {}
                Err(ReadExactError::UnexpectedEof) => return Err(Error::EndOfFile),
                Err(ReadExactError::Other(w)) => return Err(Error::Wire(w)),
            };
            cksm.update(now);

            let calc_cksm = cksm.finalize();
            let rept_cksm = u16::from_wire(serial, None).await?;

            if calc_cksm != rept_cksm {
                return Err(Error::BadChecksum);
            }
        }
        let used = ttl_len - window.len();
        Ok(used)
    }
}

// Helper macro for implementing basic Command + Acknowledge patterns.
//
// Items can optionally take send or receive payloads, though they need to
// be "owned" items, so not good for streaming.
macro_rules! cmds_with_ack {
    (
        | Function      | Code          | CmdDataTy     | RespDataTy    |
        | $(-)*         | $(-)*         | $(-)*         | $(-)*         |
     $( | $func:ident   | $code:ident   | $($cdt:ty)?   | $($rdy:ty)?   | )*
    ) => {
        $(
            #[allow(unused_parens)]
            pub async fn $func<S>(&self, serial: &mut S, $(arg: $cdt)?) -> Result<($($rdy)?), Error<S>>
            where
                S: Read + Write + ErrorType,
            {
                // Send the command
                //
                let cmd = Command {
                    address: self.address,
                    instruction: Commands::$code,
                    body: {
                        let _body = ();
                        $(
                            let _body: $cdt = arg;
                        )?
                        _body
                    },
                };
                cmd.to_wire(serial).await?;

                // Receive the data
                // TODO: Timeout?
                let resp = Response::<($($rdy)?)>::from_wire(serial).await?;

                let mut good = true;
                good &= resp.address == self.address;
                good &= resp.ident == PackageIdentifier::AcknowledgePacket.into();
                if !good {
                    return Err(Error::IncorrectData);
                }
                if resp.confirmation != ConfirmationCode::SuccessCode {
                    return Err(Error::BadConfirmation(resp.confirmation));
                }
                Ok(resp.body)
            }
        )*
    };
}

#[derive(Debug)]
pub struct LoadCharRequest {
    pub char_buffer: CharBufferId,
    pub model_id: u16,
}

impl ToWire for LoadCharRequest {
    fn size_on_wire(&self) -> usize {
        3
    }

    async fn to_wire<S: Write + ErrorType>(
        &self,
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> Result<(), Error<S>> {
        let [hi, lo] = self.model_id.to_be_bytes();
        let data = [self.char_buffer.into(), hi, lo];
        if let Some(c) = cksm {
            c.update(&data);
        }
        serial.write_all(&data).await.map_err(Error::Wire)
    }
}

impl R503 {
    cmds_with_ack! {
        | Function              | Code                      | CmdDataTy             | RespDataTy    |
        | --------              | ----                      | ---------             | ----------    |
        | get_rand_code         | GetRandomCode             |                       | u32           |
        | read_system_parameter | ReadSystemParameter       |                       | [u8; 16]      |
        | get_image             | GetImage                  |                       |               |
        | upload_image          | UpImage                   |                       |               |
        | generate_char         | GenChar                   | CharBufferId          |               |
        | generate_template     | RegModel                  |                       |               |
        | upload_template       | UpChar                    | CharBufferId          |               |
        | set_aura              | AuraControl               | AuraControlPayload    |               |
        | read_idx_table        | ReadIndexTable            | IndexTableIdx         | [u8; 32]      |
        | empty                 | Empty                     |                       |               |
        | load_char             | LoadChar                  | LoadCharRequest       |               |
    }
}

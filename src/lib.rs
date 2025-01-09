#![cfg_attr(not(any(feature = "std", test)), no_std)]

use core::fmt::Debug;

use constants::{
    AuraControlPayload, AutoEnrollStep, CharBufferId, Commands, ConfirmationCode, PackageIdentifier,
};
use embedded_io_async::{ErrorType, Read, ReadExactError, Write};
use wire_traits::{FromWire, ToWire};

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

pub struct AutoEnroll<'a, S: Read + Write + ErrorType> {
    address: u32,
    serial: &'a mut S,
}

pub struct AutoEnrollLocation {
    val: u8,
}

impl AutoEnrollLocation {
    pub fn specific(loc: u8) -> Option<Self> {
        if (0x00..0xC8).contains(&loc) {
            Some(Self { val: loc })
        } else {
            None
        }
    }

    pub fn automatic() -> Self {
        Self { val: 0xC8 }
    }
}

pub struct AutoEnrollConfig {
    /// fingerprint location
    pub location: AutoEnrollLocation,
    /// allow "cover ID number" (I don't know what this means)
    pub cover_id: bool,
    /// allow duplicate fingerprints
    pub allow_dupes: bool,
    /// "Module return the status in the critical step" (I don't know what this means)
    pub return_status: bool,
    /// "Finger have to leave in order to enter the next image collection"
    pub require_release: bool,
}

impl Default for AutoEnrollConfig {
    fn default() -> Self {
        Self {
            location: AutoEnrollLocation::automatic(),
            cover_id: false,
            allow_dupes: false,
            return_status: true,
            require_release: true,
        }
    }
}

impl ToWire for AutoEnrollConfig {
    fn size_on_wire(&self) -> usize {
        5
    }

    async fn to_wire<S: Write + ErrorType>(
        &self,
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> Result<(), Error<S>> {
        let data = [
            self.location.val,
            self.cover_id as u8,
            self.allow_dupes as u8,
            self.return_status as u8,
            self.require_release as u8,
        ];
        if let Some(c) = cksm {
            c.update(&data);
        }
        serial.write_all(&data).await.map_err(Error::Wire)
    }
}

#[derive(Debug, PartialEq)]
pub struct AutoEnrollResponse {
    pub step: AutoEnrollStep,
    pub model_id: u8,
}

impl FromWire for AutoEnrollResponse {
    async fn from_wire<S: Read + ErrorType>(
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> Result<Self, Error<S>> {
        let mut bytes = [0u8; 3];
        match serial.read_exact(&mut bytes).await {
            Ok(()) => {}
            Err(ReadExactError::UnexpectedEof) => return Err(Error::EndOfFile),
            Err(ReadExactError::Other(w)) => return Err(Error::Wire(w)),
        };

        if let Some(c) = cksm {
            c.update(&bytes);
        }

        // I'm not sure what this unused byte is for?
        let [step, _unused, id] = bytes;

        let Ok(step) = AutoEnrollStep::try_from(step) else {
            return Err(Error::IncorrectData);
        };
        // TODO: it seems like model id is returned in every step?
        let model_id = id;

        Ok(Self { step, model_id })
    }
}

impl<'a, S> AutoEnroll<'a, S>
where
    S: Read + Write + ErrorType,
{
    pub fn new(address: u32, serial: &'a mut S) -> Self {
        Self { address, serial }
    }

    /// All the steps, without yielding back control to get progress
    /// notifications
    pub async fn oneshot(mut self, cfg: AutoEnrollConfig) -> Result<u8, Error<S>> {
        self.start(cfg).await?;
        self.wait_collect_image1().await?;
        self.wait_generate_feature1().await?;
        self.wait_collect_image2().await?;
        self.wait_generate_feature2().await?;
        self.wait_collect_image3().await?;
        self.wait_generate_feature3().await?;
        self.wait_collect_image4().await?;
        self.wait_generate_feature4().await?;
        self.wait_collect_image5().await?;
        self.wait_generate_feature5().await?;
        self.wait_collect_image6().await?;
        self.wait_generate_feature6().await?;
        self.wait_repeatfingerprint().await?;
        self.wait_merge_feature().await?;
        self.wait_storage_template().await
    }

    /// Step 0
    pub async fn start(&mut self, cfg: AutoEnrollConfig) -> Result<(), Error<S>> {
        let command = Command {
            address: self.address,
            instruction: Commands::AutomaticRegistrationTemplate,
            body: cfg,
        };
        command.to_wire(self.serial).await
    }

    // 0x01: Collect image for the first time
    pub async fn wait_collect_image1(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::CollectImage1)
            .await
            .map(drop)
    }
    // 0x02: Generate Feature for the first time
    pub async fn wait_generate_feature1(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::GenerateFeature1)
            .await
            .map(drop)
    }
    // 0x03: Collect image for the second time
    pub async fn wait_collect_image2(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::CollectImage2)
            .await
            .map(drop)
    }
    // 0x04: Generate Feature for the second time
    pub async fn wait_generate_feature2(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::GenerateFeature2)
            .await
            .map(drop)
    }
    // 0x05: Collect image for the third time
    pub async fn wait_collect_image3(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::CollectImage3)
            .await
            .map(drop)
    }
    // 0x06: Generate Feature for the third time
    pub async fn wait_generate_feature3(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::GenerateFeature3)
            .await
            .map(drop)
    }
    // 0x07: Collect image for the fourth time
    pub async fn wait_collect_image4(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::CollectImage4)
            .await
            .map(drop)
    }
    // 0x08: Generate Feature for the fourth time
    pub async fn wait_generate_feature4(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::GenerateFeature4)
            .await
            .map(drop)
    }
    // 0x09: Collect image for the fifth time
    pub async fn wait_collect_image5(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::CollectImage5)
            .await
            .map(drop)
    }
    // 0x10: Generate Feature for the fifth time
    pub async fn wait_generate_feature5(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::GenerateFeature5)
            .await
            .map(drop)
    }
    // 0x11: Collect image for the sixth time
    pub async fn wait_collect_image6(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::CollectImage6)
            .await
            .map(drop)
    }
    // 0x12: Generate Feature for the sixth time
    pub async fn wait_generate_feature6(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::GenerateFeature6)
            .await
            .map(drop)
    }
    // 0x0D: Repeat fingerprint check
    pub async fn wait_repeatfingerprint(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::Repeatfingerprint)
            .await
            .map(drop)
    }
    // 0x0E: Merge feature
    pub async fn wait_merge_feature(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::MergeFeature)
            .await
            .map(drop)
    }
    // 0x0F: Storage template
    pub async fn wait_storage_template(&mut self) -> Result<u8, Error<S>> {
        self.wait_step(self.address, AutoEnrollStep::StorageTemplate)
            .await
    }

    async fn wait_step(&mut self, address: u32, step: AutoEnrollStep) -> Result<u8, Error<S>> {
        let resp = Response::<AutoEnrollResponse>::from_wire(self.serial).await?;
        let mut good = true;
        good &= resp.address == address;
        good &= resp.ident == PackageIdentifier::AcknowledgePacket.into();
        if !good {
            return Err(Error::IncorrectData);
        }
        if resp.confirmation != ConfirmationCode::SuccessCode {
            return Err(Error::BadConfirmation(resp.confirmation));
        }
        if resp.body.step != step {
            return Err(Error::IncorrectData);
        }
        Ok(resp.body.model_id)
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
    }
}

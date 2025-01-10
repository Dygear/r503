use embedded_io_async::{ErrorType, Read, Write};

use crate::{
    constants::{AutoEnrollStep, AutoIdentCount, AutoIdentifyStep, Commands, ConfirmationCode, IdentifySafety, PackageIdentifier}, wire_traits::{FromWire, ToWire}, Checksum, Command, Error, Response
};

//////////////////////////////////////////////////////////////////////////////
// Auto Enroll
//////////////////////////////////////////////////////////////////////////////

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
        let bytes = <[u8; 3]>::from_wire(serial, cksm).await?;

        // I'm not sure what this unused byte is for?
        // is the ID actually a BE u16?
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

//////////////////////////////////////////////////////////////////////////////
// Auto Identify
//////////////////////////////////////////////////////////////////////////////

pub struct AutoIdentify<'a, S: Read + Write + ErrorType> {
    address: u32,
    serial: &'a mut S,
}

#[derive(Debug, Clone)]
pub struct AutoIdentifyConfig {
    pub grade: IdentifySafety,
    pub start_pos: u8,
    pub steps_or_end: u8,
    pub return_status: bool,
    pub err_count: AutoIdentCount,
}

                // 3,    // safe grade 1-5
                // 0,    // start position
                // 8,    // unclear if this is "count" or "end position"
                // 1,    // return key steps
                // 0xFF, // attempts, I think?

impl Default for AutoIdentifyConfig {
    fn default() -> Self {
        Self {
            grade: IdentifySafety::Three,
            start_pos: 0,
            steps_or_end: 199,
            return_status: true,
            err_count: AutoIdentCount::TimesWithTimeout(0xFF),
        }
    }
}

impl ToWire for AutoIdentifyConfig {
    fn size_on_wire(&self) -> usize {
        5
    }

    async fn to_wire<S: Write + ErrorType>(
        &self,
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> Result<(), Error<S>> {
        let data = [
            self.grade.into(),
            self.start_pos,
            self.steps_or_end,
            self.return_status as u8,
            self.err_count.into(),
        ];
        if let Some(c) = cksm {
            c.update(&data);
        }
        serial.write_all(&data).await.map_err(Error::Wire)
    }
}

#[derive(Debug, PartialEq)]
pub struct AutoIdentifyResponse {
    pub step: AutoIdentifyStep,
    pub model_id: u8,
    pub score: u16,
}

impl FromWire for AutoIdentifyResponse {
    async fn from_wire<S: Read + ErrorType>(
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> Result<Self, Error<S>> {
        let bytes = <[u8; 5]>::from_wire(serial, cksm).await?;

        // I'm not sure what this unused byte is for?
        // is the ID actually a BE u16?
        let [step, _unused, id, score_hi, score_lo] = bytes;

        let Ok(step) = AutoIdentifyStep::try_from(step) else {
            return Err(Error::IncorrectData);
        };
        // TODO: it seems like model id is returned in every step?
        let model_id = id;
        let score = u16::from_be_bytes([score_hi, score_lo]);

        Ok(Self { step, model_id, score })
    }
}

impl<'a, S> AutoIdentify<'a, S>
where
    S: Read + Write + ErrorType,
{
    pub fn new(address: u32, serial: &'a mut S) -> Self {
        Self { address, serial }
    }

    /// Step 0
    pub async fn start(&mut self, cfg: AutoIdentifyConfig) -> Result<(), Error<S>> {
        let command = Command {
            address: self.address,
            instruction: Commands::AutomaticFingerprintVerification,
            body: cfg,
        };
        command.to_wire(self.serial).await
    }

    pub async fn wait_collect_image(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoIdentifyStep::CollectImage).await.map(drop)
    }

    pub async fn wait_generate_feature(&mut self) -> Result<(), Error<S>> {
        self.wait_step(self.address, AutoIdentifyStep::GenerateFeature).await.map(drop)
    }

    pub async fn wait_search(&mut self) -> Result<AutoIdentifyResponse, Error<S>> {
        self.wait_step(self.address, AutoIdentifyStep::Search).await
    }

    async fn wait_step(&mut self, address: u32, step: AutoIdentifyStep) -> Result<AutoIdentifyResponse, Error<S>> {
        let resp = Response::<AutoIdentifyResponse>::from_wire(self.serial).await?;
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
        Ok(resp.body)
    }
}

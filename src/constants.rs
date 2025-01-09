use core::num::NonZeroU8;

use crate::{wire_traits::ToWire, Error};

// Helper macro that generate a lot of accessors for enum to integer conversions
macro_rules! be_enum {
    (
        name: $enum_name:ident;
        integer: $int_ty:ty;
        {
            $(
                $var_name:ident -> $var_val:literal,
            )+
        }
    ) => {
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub enum $enum_name {
            $(
                $var_name,
            )+
        }

        impl From<$enum_name> for $int_ty {
            fn from(value: $enum_name) -> Self {
                match value {
                    $(
                        $enum_name::$var_name => $var_val,
                    )+
                }
            }
        }

        impl TryFrom<$int_ty> for $enum_name {
            type Error = $int_ty;
            fn try_from(value: $int_ty) -> Result<Self, Self::Error> {
                match value {
                    $(
                        $var_val => Ok($enum_name::$var_name),
                    )+
                    other => Err(other),
                }
            }
        }

        impl crate::ToWire for $enum_name {
            fn size_on_wire(&self) -> usize {
                size_of::<$int_ty>()
            }

            async fn to_wire<S: embedded_io_async::Write + embedded_io_async::ErrorType>(
                &self,
                serial: &mut S,
                cksm: Option<&mut crate::Checksum>,
            ) -> Result<(), crate::Error<S>> {
                let val: $int_ty = (*self).into();
                val.to_wire(serial, cksm).await
            }
        }

        impl crate::FromWire for $enum_name {
            async fn from_wire<S: embedded_io_async::Read + embedded_io_async::ErrorType>(
                serial: &mut S,
                cksm: Option<&mut crate::Checksum>,
            ) -> Result<Self, crate::Error<S>> {
                let val = <$int_ty>::from_wire(serial, cksm).await?;
                match Self::try_from(val) {
                    Ok(v) => Ok(v),
                    Err(_) => Err(crate::Error::IncorrectData)
                }
            }
        }
    };
}

// Package Identifier Field
be_enum! {
    name: PackageIdentifier;
    integer: u8;
    {
        CommandPacket -> 0x01,
        DataPacket -> 0x02,
        AcknowledgePacket -> 0x07,
        EndOfDataPacket -> 0x08,
    }
}

// Command ID field
be_enum! {
    name: Commands;
    integer: u8;
    {
        GetImage -> 0x01,
        GenChar -> 0x02,
        RegModel -> 0x05,
        UpChar -> 0x08,
        UpImage -> 0x0A,
        GetRandomCode -> 0x14,
        ReadSystemParameter -> 0x0F,
        AutomaticRegistrationTemplate -> 0x31,
        AuraControl -> 0x35,
    }
}

// Confirmation (and error) Code field
be_enum! {
    name: ConfirmationCode;
    integer: u8;
    {
        // 0 = Commad Execution Complete;
        SuccessCode -> 0x00,
        // 1 = Error When Receiving Data Package;
        ErrorCode -> 0x01,
        // 2 -> No Finger on the Sensor;
        NoFingerOnSensor -> 0x02,
        // 3 -> Fail to Enroll the Finger;
        FailToEnrollFinger -> 0x03,
        // 6 -> Fail to Generate Character File Due to the Over-disorderly Fingerprint Image;
        FailToGenerateCharacterOverDisorderlyFingerprintImage -> 0x06,
        // 7 -> Fail to Generate Character File due to Lackness of Character Point or Over-smallness of Fingerprint Image;
        FailToGenerateCharacterLacknessOfCharacterPointOrOverSmallness -> 0x07,
        // 8 -> Finger Doesn't Match;
        FailFingerDoesntMatch -> 0x08,
        // 9 -> Fail to Find the Matching Finger;
        FailToFindMatchingFinger -> 0x09,
        // 10 -> Fail to Combine the Character Files;
        FailToCombineCharacterFiles -> 0x0A,
        // 11 -> Addressing PageID is Beyond the Finger Library;
        AddressingPageIDIsBeyoundTheFingerLibary -> 0x0B,
        // 12 -> Error When Reading Template from Library or the Template is Invalid;
        ErrorWhenReadingTemplateFromLibararORTemplateIsInvalid -> 0x0C,
        // 13 -> Error When Uploading Template;
        ErrorWhenUploadingTemplate -> 0x0D,
        // 14 -> Module can't receive the following data packages;
        ModuleCantReceivingTheFollowingDataPackages -> 0x0E,
        // 15 -> Error when uploading image;
        ErrorWhenUploadingImage -> 0x0F,
        // 16 -> Fail to delete the template;
        FailToDeleteTheTemplate -> 0x10,
        // 17 -> Fail to clear finger library;
        FailToClearFingerLibary -> 0x11,
        // 19 -> Wrong password;
        WrongPassword -> 0x13,
        // 21 -> Fail to generate the image for the lackness of valid primary image;
        FailToGenerateImageLacknessOfValidPrimaryImage -> 0x15,
        // 24 -> Error when writing flash;
        ErrorWhenWritingFlash -> 0x18,
        // 25 -> No definition error;
        NoDefinitionError -> 0x19,
        // 26 -> Invalid register number;
        InvalidRegisterNumber -> 0x1A,
        // 27 -> Incorrect configuration of register;
        IncorrectConfigurationOfRegister -> 0x1B,
        // 28 -> Wrong notepad page number;
        WrongNotepadPageNumber -> 0x1C,
        // 29 -> Fail to operate the communication port;
        FailToOperateTheCommunicationPort -> 0x1D,
        // 31 -> The fingerprint libary is full;
        FingerPrintLibaryFull -> 0x1F,
        // 32 -> The address code is incorrect;
        AddressIncorrect -> 0x20,
        // 33 -> The password must be verified;
        MustVerifyPassword -> 0x21,
        // 34 -> The fingerprint template is empty;
        FingerTemplateEmpty -> 0x22,
        // 36 -> The fingerprint library is empty;
        FingerLibaryEmpty -> 0x24,
        // 38 -> Timeout;
        Timeout -> 0x26,
        // 39 -> The fingerprints already exist;
        FingerAlreadyExists -> 0x27,
        // 41 Sensor hardware error;
        SensorHardwareError -> 0x29,
        // 252 -> Unsupported command;
        UnsupportedCommand -> 0xFC,
        // 253 -> Hardware Error;
        HardwareError -> 0xFD,
        // 254 -> Command execution failure;
        CommandExecutionFailure -> 0xFE,
        // 255 Others: System Reserved; (And Default for this Rust Lib);
        SystemReserved -> 0xFF,
    }
}

// Char Buffer ID field
be_enum! {
    name: CharBufferId;
    integer: u8;
    {
        One -> 0x01,
        Two -> 0x02,
        Three -> 0x03,
        Four -> 0x04,
        Five -> 0x05,
        Six -> 0x06,
    }
}


be_enum! {
    name: AutoEnrollStep;
    integer: u8;
    {
        // 0x01: Collect image for the first time
        CollectImage1 -> 0x01,
        // 0x02: Generate Feature for the first time
        GenerateFeature1 -> 0x02,
        // 0x03: Collect image for the second time
        CollectImage2 -> 0x03,
        // 0x04: Generate Feature for the second time
        GenerateFeature2 -> 0x04,
        // 0x05: Collect image for the third time
        CollectImage3 -> 0x05,
        // 0x06: Generate Feature for the third time
        GenerateFeature3 -> 0x06,
        // 0x07: Collect image for the fourth time
        CollectImage4 -> 0x07,
        // 0x08: Generate Feature for the fourth time
        GenerateFeature4 -> 0x08,
        // 0x09: Collect image for the fifth time
        CollectImage5 -> 0x09,
        // 0x0A: Generate Feature for the fifth time
        GenerateFeature5 -> 0x0A,
        // 0x0B: Collect image for the sixth time
        CollectImage6 -> 0x0B,
        // 0x0C: Generate Feature for the sixth time
        GenerateFeature6 -> 0x0C,
        // 0x0D: Repeat fingerprint check
        Repeatfingerprint -> 0x0D,
        // 0x0E: Merge feature
        MergeFeature -> 0x0E,
        // 0x0F: Storage template
        StorageTemplate -> 0x0F,
    }
}

be_enum! {
    name: AuraControlCode;
    integer: u8;
    {
        Breathing -> 0x01,
        Flashing -> 0x02,
        AlwaysOn -> 0x03,
        AlwaysOff -> 0x04,
        GraduallyOn -> 0x05,
        GraduallyOff -> 0x06,
    }
}

pub type AuraSpeed = u8;

be_enum! {
    name: AuraColorIndex;
    integer: u8;
    {
        Red -> 0x01,
        Blue -> 0x02,
        Purple -> 0x03,
        Green -> 0x04,
        Yellow -> 0x05,
        Cyan -> 0x06,
        White -> 0x07,
    }
}

#[derive(Debug)]
pub enum AuraCycleCount {
    Infinite,
    Times(u8),
}

impl ToWire for AuraCycleCount {
    fn size_on_wire(&self) -> usize {
        1
    }

    async fn to_wire<S: embedded_io_async::Write + embedded_io_async::ErrorType>(
        &self,
        serial: &mut S,
        cksm: Option<&mut crate::Checksum>,
    ) -> Result<(), Error<S>> {
        let value: u8 = match self {
            AuraCycleCount::Infinite => 0,
            AuraCycleCount::Times(non_zero) => *non_zero,
        };
        if let Some(c) = cksm {
            c.update(&[value]);
        }
        serial.write_all(&[value]).await.map_err(Error::Wire)
    }
}

#[derive(Debug)]
pub struct AuraControlPayload {
    pub ctrl_code: AuraControlCode,
    pub speed: AuraSpeed,
    pub color: AuraColorIndex,
    pub count: AuraCycleCount,
}

impl ToWire for AuraControlPayload {
    fn size_on_wire(&self) -> usize {
        4
    }

    async fn to_wire<S: embedded_io_async::Write + embedded_io_async::ErrorType>(
        &self,
        serial: &mut S,
        cksm: Option<&mut crate::Checksum>,
    ) -> Result<(), Error<S>> {
        // this is awkward
        if let Some(cksm) = cksm {
            self.ctrl_code.to_wire(serial, Some(cksm)).await?;
            self.speed.to_wire(serial, Some(cksm)).await?;
            self.color.to_wire(serial, Some(cksm)).await?;
            self.count.to_wire(serial, Some(cksm)).await?;
        } else {
            self.ctrl_code.to_wire(serial, None).await?;
            self.speed.to_wire(serial, None).await?;
            self.color.to_wire(serial, None).await?;
            self.count.to_wire(serial, None).await?;
        }
        Ok(())
    }
}

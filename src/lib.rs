#![cfg_attr(not(any(feature = "std", test)), no_std)]

use core::fmt::Debug;

use embedded_io_async::{ErrorType, Read, ReadExactError, Write};

pub struct R503 {
    address: u32,
}

impl R503 {
    pub fn new_with_address(addr: u32) -> Self {
        Self { address: addr }
    }
}

pub enum Error<S>
where
    S: ErrorType,
{
    Wire(S::Error),
    IncorrectData,
    EndOfFile,
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
        }
    }
}

impl R503 {
    pub async fn get_rand_code<S>(&self, serial: &mut S) -> Result<u32, Error<S>>
    where
        S: Read + Write + ErrorType,
    {
        // Send the command
        //
        let tx_fut = async {
            // Header
            serial.write_all(&[0xEF, 0x01]).await?;
            // Adder
            serial.write_all(&self.address.to_be_bytes()).await?;

            // CRC starts here!
            let mut crc = Checksum::new();

            // Package Identifier
            let ident = PackageIdentifier::CommandPacket.to_bytes();
            crc.update(&ident);
            serial.write_all(&ident).await?;

            // length
            let length = 3u16.to_be_bytes(); // command + CRC
            crc.update(&length);
            serial.write_all(&length).await?;

            // command
            let cmd = Commands::GetRandomCode.to_bytes();
            crc.update(&cmd);
            serial.write_all(&cmd).await?;

            // CRC
            serial.write_all(&crc.finalize().to_be_bytes()).await?;

            Ok(())
        };
        tx_fut.await.map_err(Error::Wire)?;

        // Receive the data
        // TODO: Timeout?
        // total size is 16 bytes
        let mut rx_buf = [0u8; 16];
        match serial.read_exact(&mut rx_buf).await {
            Ok(()) => {}
            Err(ReadExactError::UnexpectedEof) => return Err(Error::EndOfFile),
            Err(ReadExactError::Other(w)) => return Err(Error::Wire(w)),
        };
        let resp = Response::from_slice(&rx_buf)?;
        let mut good = true;
        good &= resp.address == self.address;
        good &= resp.ident == PackageIdentifier::AcknowledgePacket.into();
        good &= [resp.confirmation] == ConfirmationCode::SuccessCode.to_bytes();
        good &= resp.body.len() == 4;
        if !good {
            return Err(Error::IncorrectData);
        }
        Ok(take_u32::<S>(resp.body)?.0)
    }
}

// pub struct Command<'a> {
//     address: u32,
//     instruction: u8,
//     body: &'a [u8],
// }

pub struct Response<'a> {
    address: u32,
    ident: u8,
    confirmation: u8,
    body: &'a [u8],
}

impl<'a> Response<'a> {
    pub fn from_slice<S: ErrorType>(sli: &'a [u8]) -> Result<Self, Error<S>> {
        // Do we have the right header?
        let remain = take_header::<S>(sli)?;
        let (address, remain) = take_u32::<S>(remain)?;

        // The remaining bits are checksum relevant!
        let temp_len = remain.len();
        if remain.len() < 2 {
            return Err(Error::IncorrectData);
        }
        let (remain, cksm) = remain.split_at(temp_len - 2);
        // does the checksum check?
        let mut check = Checksum::new();
        check.update(remain);
        let calc = check.finalize();
        if calc.to_be_bytes() != cksm {
            return Err(Error::IncorrectData);
        }

        let (ident, remain) = take_u8::<S>(remain)?;
        let (len, remain) = take_u16::<S>(remain)?;
        let (confirmation, remain) = take_u8::<S>(remain)?;
        let len_usize = len as usize;
        // Does length match, counting the CRC and confirmation bytes?
        if (remain.len() + 3) != len_usize {
            return Err(Error::IncorrectData);
        }
        Ok(Self {
            address,
            ident,
            confirmation,
            body: remain,
        })
    }
}

fn take_header<S: ErrorType>(sli: &[u8]) -> Result<&[u8], Error<S>> {
    if sli.len() < 2 {
        return Err(Error::IncorrectData);
    }
    let (now, later) = sli.split_at(2);
    if now != [0xEF, 0x01] {
        return Err(Error::IncorrectData);
    }
    Ok(later)
}

fn take_u8<S: ErrorType>(sli: &[u8]) -> Result<(u8, &[u8]), Error<S>> {
    let (now, later) = sli.split_first().ok_or(Error::IncorrectData)?;
    Ok((*now, later))
}

fn take_u16<S: ErrorType>(sli: &[u8]) -> Result<(u16, &[u8]), Error<S>> {
    if sli.len() < 2 {
        return Err(Error::IncorrectData);
    }
    let (now, later) = sli.split_at(2);
    let mut buf = [0u8; 2];
    buf.copy_from_slice(now);
    Ok((u16::from_be_bytes(buf), later))
}

fn take_u32<S: ErrorType>(sli: &[u8]) -> Result<(u32, &[u8]), Error<S>> {
    if sli.len() < 4 {
        return Err(Error::IncorrectData);
    }
    let (now, later) = sli.split_at(4);
    let mut buf = [0u8; 4];
    buf.copy_from_slice(now);
    Ok((u32::from_be_bytes(buf), later))
}

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

        impl $enum_name {
            pub fn to_bytes(&self) -> [u8; size_of::<$int_ty>()] {
                <$int_ty>::from(*self).to_be_bytes()
            }

            pub fn try_from_slice(&self, sli: &[u8]) -> Option<Self> {
                if sli.len() != size_of::<$int_ty>() {
                    return None;
                }
                let mut buf = [0u8; size_of::<$int_ty>()];
                buf.copy_from_slice(sli);
                let val = <$int_ty>::from_be_bytes(buf);
                Self::try_from(val).ok()
            }
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
    };
}

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

be_enum! {
    name: Commands;
    integer: u8;
    {
        GetRandomCode -> 0x14,
    }
}

be_enum! {
    name: ConfirmationCode;
    integer: u8;
    {
        SuccessCode -> 0x00,
        ErrorCode -> 0x01,
    }
}

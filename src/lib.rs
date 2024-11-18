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

pub struct Command<T: ToWire> {
    address: u32,
    instruction: Commands,
    body: T,
}

impl<T: ToWire> Command<T> {
    pub async fn to_wire<S>(&self, serial: &mut S) -> Result<(), Error<S>>
    where
        S: Read + Write + ErrorType,
    {
        // Header
        0xEF01u16.to_wire(serial, None).await?;
        // Adder
        self.address.to_wire(serial, None).await?;

        // CRC starts here!
        let mut crc = Checksum::new();

        // Package Identifier
        PackageIdentifier::CommandPacket.to_wire(serial, Some(&mut crc)).await?;

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
            return Err(Error::IncorrectData);
        }
        Ok(Self {
            address,
            ident,
            confirmation,
            body,
        })
    }
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

        impl ToWire for $enum_name {
            fn size_on_wire(&self) -> usize {
                size_of::<$int_ty>()
            }

            async fn to_wire<S: Write + ErrorType>(
                &self,
                serial: &mut S,
                cksm: Option<&mut Checksum>,
            ) -> Result<(), Error<S>> {
                let val: $int_ty = (*self).into();
                val.to_wire(serial, cksm).await
            }
        }

        impl FromWire for $enum_name {
            async fn from_wire<S: Read + ErrorType>(
                serial: &mut S,
                cksm: Option<&mut Checksum>,
            ) -> Result<Self, Error<S>> {
                let val = <$int_ty>::from_wire(serial, cksm).await?;
                match Self::try_from(val) {
                    Ok(v) => Ok(v),
                    Err(_) => Err(Error::IncorrectData)
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

pub trait ToWire {
    fn size_on_wire(&self) -> usize;
    fn to_wire<S: Write + ErrorType>(
        &self,
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> impl core::future::Future<Output = Result<(), Error<S>>>;
}

impl ToWire for [u8] {
    fn size_on_wire(&self) -> usize {
        self.len()
    }

    async fn to_wire<S: Write + ErrorType>(
        &self,
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> Result<(), Error<S>> {
        if let Some(cksm) = cksm {
            cksm.update(self);
        }
        serial.write_all(self).await.map_err(Error::Wire)
    }
}

macro_rules! wire_ints {
    ($($int_ty:ty),*) => {
        $(
            impl ToWire for $int_ty {
                fn size_on_wire(&self) -> usize {
                    size_of::<$int_ty>()
                }

                async fn to_wire<S: Write + ErrorType>(
                    &self,
                    serial: &mut S,
                    cksm: Option<&mut Checksum>,
                ) -> Result<(), Error<S>> {
                    let bytes = (*self).to_be_bytes();
                    if let Some(cksm) = cksm {
                        cksm.update(&bytes);
                    }
                    serial.write_all(&bytes).await.map_err(Error::Wire)
                }
            }

            impl FromWire for $int_ty {
                async fn from_wire<S: Read + ErrorType>(
                    serial: &mut S,
                    cksm: Option<&mut Checksum>,
                ) -> Result<Self, Error<S>> {
                    let mut buf = [0u8; size_of::<$int_ty>()];
                    match serial.read_exact(&mut buf).await {
                        Ok(()) => {}
                        Err(ReadExactError::UnexpectedEof) => return Err(Error::EndOfFile),
                        Err(ReadExactError::Other(w)) => return Err(Error::Wire(w)),
                    };
                    if let Some(cksm) = cksm {
                        cksm.update(&buf);
                    }
                    Ok(<$int_ty>::from_be_bytes(buf))
                }
            }
        )*
    };
}

wire_ints!(u8, u16, u32);

impl ToWire for () {
    fn size_on_wire(&self) -> usize {
        0
    }

    async fn to_wire<S: Write + ErrorType>(
        &self,
        _serial: &mut S,
        _cksm: Option<&mut Checksum>,
    ) -> Result<(), Error<S>> {
        Ok(())
    }
}

impl FromWire for () {
    async fn from_wire<S: Read + ErrorType>(
        _serial: &mut S,
        _cksm: Option<&mut Checksum>,
    ) -> Result<Self, Error<S>> {
        Ok(())
    }
}

pub trait FromWire: Sized {
    fn from_wire<S: Read + ErrorType>(
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> impl core::future::Future<Output = Result<Self, Error<S>>>;
}

macro_rules! cmds_with_ack {
    (
        | Function      | Code          | CmdDataTy     | RespDataTy    |
        | $(-)*         | $(-)*         | $(-)*         | $(-)*         |
        | $func:ident   | $code:ident | $($cdt:ty)?   | $($rdy:ty)?   |
    ) => {
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
            //
            // We expect 4 data bytes back
            let resp = Response::<($($rdy)?)>::from_wire(serial).await?;

            let mut good = true;
            good &= resp.address == self.address;
            good &= resp.ident == PackageIdentifier::AcknowledgePacket.into();
            good &= resp.confirmation == ConfirmationCode::SuccessCode;
            if !good {
                return Err(Error::IncorrectData);
            }
            Ok(resp.body)
        }
    };
}

impl R503 {
    cmds_with_ack!{
        | Function          | Code                      | CmdDataTy         | RespDataTy    |
        | --------          | ----                      | ---------         | ----------    |
        | get_rand_code     | GetRandomCode             |                   | u32           |
    }
}

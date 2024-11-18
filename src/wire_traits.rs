use embedded_io_async::{ErrorType, Read, ReadExactError, Write};

use crate::{Checksum, Error};

/// Serialize the data TO the wire.
pub trait ToWire {
    /// Size on the wire, in bytes. Must be known ahead of time.
    fn size_on_wire(&self) -> usize;

    /// Serialize the data to the wire
    ///
    /// Implementers MUST update the checksum field if it is present.
    ///
    /// Implementers MUST serialize the same number of bytes as reported by
    /// Self::size_on_wire
    fn to_wire<S: Write + ErrorType>(
        &self,
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> impl core::future::Future<Output = Result<(), Error<S>>>;
}

/// Deserialize data FROM the wire
pub trait FromWire: Sized {
    fn from_wire<S: Read + ErrorType>(
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> impl core::future::Future<Output = Result<Self, Error<S>>>;
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

// Macro for implementing traits for basic integer types
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

// Implement traits for basic wire types
wire_ints!(u8, u16, u32);

// When you send nothing, you send ()
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

// When you receive nothing, you receive ()
impl FromWire for () {
    async fn from_wire<S: Read + ErrorType>(
        _serial: &mut S,
        _cksm: Option<&mut Checksum>,
    ) -> Result<Self, Error<S>> {
        Ok(())
    }
}


impl<const N: usize> ToWire for [u8; N] {
    fn size_on_wire(&self) -> usize {
        N
    }

    async fn to_wire<S: Write + ErrorType>(
        &self,
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> Result<(), Error<S>> {
        self.as_slice().to_wire(serial, cksm).await
    }
}

impl<const N: usize> FromWire for [u8; N] {
    async fn from_wire<S: Read + ErrorType>(
        serial: &mut S,
        cksm: Option<&mut Checksum>,
    ) -> Result<Self, Error<S>> {
        let mut buf = [0u8; N];
        match serial.read_exact(&mut buf).await {
            Ok(()) => {}
            Err(ReadExactError::UnexpectedEof) => return Err(Error::EndOfFile),
            Err(ReadExactError::Other(w)) => return Err(Error::Wire(w)),
        };
        if let Some(cksm) = cksm {
            cksm.update(&buf);
        }
        Ok(buf)
    }
}

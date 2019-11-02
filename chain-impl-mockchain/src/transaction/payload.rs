use chain_core::{
    mempack::{ReadBuf, ReadError, Readable},
    property,
};

use crate::certificate::CertificateSlice;
use std::marker::PhantomData;

pub trait Payload: Readable {
    const HAS_DATA: bool;
    const HAS_AUTH: bool;
    type Auth: Readable;

    fn payload_data(&self) -> PayloadData<Self>;

    fn payload_auth_data(auth: &Self::Auth) -> PayloadAuthData<Self>;

    fn to_certificate_slice<'a>(p: PayloadSlice<'a, Self>) -> Option<CertificateSlice<'a>>;
}

/// Owned binary representation of a payload
pub struct PayloadData<P: ?Sized>(pub(crate) Box<[u8]>, pub(crate) PhantomData<P>);

/// Owned binary representation of a payload auth
pub struct PayloadAuthData<P: ?Sized>(pub(crate) Box<[u8]>, pub(crate) PhantomData<P>);

/// Borrowed binary representation of a payload
pub struct PayloadSlice<'a, P: ?Sized>(pub(crate) &'a [u8], pub(crate) PhantomData<P>);

/// Borrowed binary representation of a payload auth
pub struct PayloadAuthSlice<'a, P: ?Sized>(pub(crate) &'a [u8], pub(crate) PhantomData<P>);

impl<P: ?Sized> Clone for PayloadData<P> {
    fn clone(&self) -> Self {
        PayloadData(self.0.clone(), self.1.clone())
    }
}

impl<P: ?Sized> Clone for PayloadAuthData<P> {
    fn clone(&self) -> Self {
        PayloadAuthData(self.0.clone(), self.1.clone())
    }
}

impl<P: ?Sized> PayloadData<P> {
    pub fn borrow<'a>(&'a self) -> PayloadSlice<'a, P> {
        PayloadSlice(&self.0[..], self.1.clone())
    }
}

impl<P: ?Sized> PayloadAuthData<P> {
    pub fn borrow<'a>(&'a self) -> PayloadAuthSlice<'a, P> {
        PayloadAuthSlice(&self.0[..], self.1.clone())
    }
}

impl<'a, P: ?Sized> Clone for PayloadSlice<'a, P> {
    fn clone(&self) -> PayloadSlice<'a, P> {
        PayloadSlice(self.0.clone(), self.1.clone())
    }
}

impl<'a, P: ?Sized> Clone for PayloadAuthSlice<'a, P> {
    fn clone(&self) -> PayloadAuthSlice<'a, P> {
        PayloadAuthSlice(self.0.clone(), self.1.clone())
    }
}

impl<'a, P: Payload> PayloadSlice<'a, P> {
    pub fn into_payload(self) -> P {
        P::read(&mut ReadBuf::from(self.0)).unwrap()
    }
}

impl<'a, P: Payload> PayloadAuthSlice<'a, P> {
    pub fn into_payload_auth(self) -> P::Auth {
        P::Auth::read(&mut ReadBuf::from(self.0)).unwrap()
    }
}

impl<'a, P: Payload> PayloadSlice<'a, P> {
    pub fn to_owned(&self) -> PayloadData<P> {
        PayloadData(self.0.to_owned().into(), self.1.clone())
    }
}

impl<'a, P: Payload> PayloadAuthSlice<'a, P> {
    pub fn to_owned(&self) -> PayloadAuthData<P> {
        PayloadAuthData(self.0.to_owned().into(), self.1.clone())
    }
}

impl<'a, P: Payload> PayloadSlice<'a, P> {
    pub fn to_certificate_slice(self) -> Option<CertificateSlice<'a>> {
        <P as Payload>::to_certificate_slice(self)
    }
}

impl<P: ?Sized> AsRef<[u8]> for PayloadData<P> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<P: ?Sized> AsRef<[u8]> for PayloadAuthData<P> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<'a, P: ?Sized> PayloadSlice<'a, P> {
    pub fn as_bytes(&self) -> &'a [u8] {
        self.0
    }
}

impl<'a, P: ?Sized> PayloadAuthSlice<'a, P> {
    pub fn as_bytes(&self) -> &'a [u8] {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoExtra;

impl property::Serialize for NoExtra {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, _: W) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl property::Deserialize for NoExtra {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(_: R) -> Result<Self, Self::Error> {
        Ok(NoExtra)
    }
}
impl Readable for NoExtra {
    fn read<'a>(_: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(NoExtra)
    }
}
impl Payload for NoExtra {
    const HAS_DATA: bool = false;
    const HAS_AUTH: bool = false;
    type Auth = ();

    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(Vec::with_capacity(0).into(), PhantomData)
    }

    fn payload_auth_data(_: &Self::Auth) -> PayloadAuthData<Self> {
        PayloadAuthData(Vec::with_capacity(0).into(), PhantomData)
    }

    fn to_certificate_slice<'a>(_: PayloadSlice<'a, Self>) -> Option<CertificateSlice<'a>> {
        None
    }
}

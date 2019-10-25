use chain_core::{
    mempack::{ReadBuf, ReadError, Readable},
    property,
};

pub trait Payload: Readable {
    const HAS_DATA: bool;
    const HAS_AUTH: bool;
    type Auth: Readable;

    fn to_bytes(&self) -> Vec<u8>;

    fn auth_to_bytes(auth: &Self::Auth) -> Vec<u8>;
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

    fn to_bytes(&self) -> Vec<u8> {
        Vec::with_capacity(0)
    }
    fn auth_to_bytes(_: &Self::Auth) -> Vec<u8> {
        Vec::with_capacity(0)
    }
}

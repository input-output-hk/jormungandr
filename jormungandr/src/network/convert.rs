use chain_core::mempack::{ReadBuf, Readable};
use chain_network::error::{Code, Error};
use crate::blockcfg::HeaderHash;
use chain_network::data::{BlockId, BlockIds};
use chain_core::property::Serialize;
use chain_impl_mockchain::block::Header;
use chain_impl_mockchain::fragment::Fragment;

pub fn deserialize<T, U>(src: &T) -> Result<U, Error>
where
    T: AsRef<[u8]>,
    U: Readable,
{
    let mut buf = ReadBuf::from(src.as_ref());
    U::read(&mut buf).map_err(|e| Error::new(Code::InvalidArgument, e))
}

pub fn deserialize_vec<T, U>(src: &[T]) -> Result<Vec<U>, Error>
where
    T: AsRef<[u8]>,
    U: Readable,
{
    src.iter().map(|item| deserialize(item)).collect()
}

// TODO: Check if this is completly compatible
impl From<HeaderHash> for BlockId {
    fn from(header_hash: HeaderHash) -> Self {
        BlockId::try_from(header_hash.serialize_as_vec().unwrap().as_slice()).unwrap()
    }
}

// TODO: Check if this is completly compatible
impl From<chain_impl_mockchain::header::Header> for chain_network::data::block::Header {
    fn from(header: Header) -> Self {
        chain_network::data::block::Header::from_bytes(header.as_slice())
    }
}

impl From<chain_impl_mockchain::fragment::Fragment> for chain_network::data::fragment::Fragment {
    fn from(fragment: Fragment) -> Self {
        chain_network::data::fragment::Fragment::from_bytes(fragment.serialize_as_vec().unwrap())
    }
}
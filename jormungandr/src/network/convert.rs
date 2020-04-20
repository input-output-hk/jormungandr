use crate::blockcfg::{Fragment, Header, HeaderHash, HeaderId};
use chain_core::mempack::{ReadBuf, Readable};
use chain_core::property::Serialize;
use chain_network::data as net_data;
use chain_network::data::{block, fragment, BlockId, BlockIds};
use chain_network::error::{Code, Error};

fn read<T, U>(src: &T) -> Result<U, Error>
where
    T: AsRef<[u8]>,
    U: Readable,
{
    let mut buf = ReadBuf::from(src.as_ref());
    U::read(&mut buf).map_err(|e| Error::new(Code::InvalidArgument, e))
}

fn read_vec<T, U>(src: &[T]) -> Result<Vec<U>, Error>
where
    T: AsRef<[u8]>,
    U: Readable,
{
    src.iter().map(|item| read(item)).collect()
}

pub trait TryFromNetwork<N> {
    fn try_from_network(data: N) -> Result<Self, Error>;
}

pub trait IntoNetwork<N> {
    fn into_network(self) -> N;
}

impl TryFromNetwork<net_data::Fragment> for Fragment {
    fn try_from_network(data: net_data::Fragment) -> Result<Self, Error> {
        read(&data)
    }
}

// TODO: Check if this is completly compatible
impl TryFromNetwork<HeaderHash> for BlockId {
    fn try_from_network(header_hash: HeaderHash) -> Result<Self, Error> {
        BlockId::try_from(header_hash.serialize_as_vec().unwrap().as_slice())
    }
}

impl TryFromNetwork<Vec<HeaderHash>> for BlockIds {
    fn try_from_network(block_ids: Vec<HeaderHash>) -> Result<Self, Error> {
        block::try_ids_from_iter(block_ids.iter())
    }
}

// TODO: Check if this is completly compatible
impl TryFromNetwork<Header> for block::Header {
    fn try_from_network(header: Header) -> Result<Self, Error> {
        Ok(block::Header::from_bytes(header.as_slice()))
    }
}

impl TryFromNetwork<Fragment> for fragment::Fragment {
    fn try_from_network(fragment: Fragment) -> Result<Self, Error> {
        Ok(fragment::Fragment::from_bytes(fragment.serialize_as_vec()?))
    }
}

use super::p2p::Gossip;
use crate::blockcfg::{Block, Fragment, Header, HeaderId};
use chain_core::mempack::{ReadBuf, Readable};
use chain_core::property::{Deserialize, Serialize};
use chain_network::data as net_data;
use chain_network::error::{Code, Error};
use chain_impl_mockchain::fragment::FragmentRaw;
use chain_impl_mockchain::key;
use crate::blockchain::Checkpoints;
use chain_network::data::block;

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

/// Conversion from a chain-network byte container data type
/// to an application data object.
pub trait Decode {
    type Object;

    fn decode(self) -> Result<Self::Object, Error>;
}

/// Conversion from an application data object
/// to a chain-network byte container data type.
pub trait Encode {
    type NetworkData;

    fn encode(self) -> Self::NetworkData;
}

impl<T, N> Decode for Box<[N]>
where
    N: Decode<Object = T>,
{
    type Object = Vec<T>;

    fn decode(self) -> Result<Vec<T>, Error> {
        self.into_vec().into_iter().map(Decode::decode).collect()
    }
}

impl Decode for net_data::BlockId {
    type Object = HeaderId;

    fn decode(self) -> Result<Self::Object, Error> {
        read(&self)
    }
}

impl Decode for net_data::Block {
    type Object = Block;

    fn decode(self) -> Result<Self::Object, Error> {
        read(&self)
    }
}

impl Decode for net_data::Fragment {
    type Object = Fragment;

    fn decode(self) -> Result<Self::Object, Error> {
        read(&self)
    }
}

impl Encode for Fragment {
    type NetworkData = net_data::fragment::Fragment;

    fn encode(self) -> Self::NetworkData {
        net_data::fragment::Fragment::from_bytes(self.serialize_as_vec().unwrap())
    }
}

impl Decode for net_data::gossip::Node {
    type Object = Gossip;
    fn decode(self) -> Result<Self::Object, Error> {
        Gossip::deserialize(self.as_bytes()).map_err(|e| Error::new(Code::InvalidArgument, e))
    }
}

impl Encode for Gossip {
    type NetworkData = net_data::gossip::Node;

    fn encode(self) -> Self::NetworkData {
        let bytes = self.serialize_as_vec().unwrap();
        net_data::gossip::Node::from_bytes(bytes)
    }
}

impl Encode for Header {
    type NetworkData = net_data::Header;

    fn encode(self) -> Self::NetworkData {
        net_data::Header::from_bytes(self.as_slice())
    }
}

impl Encode for key::Hash {
    type NetworkData = net_data::BlockId;

    fn encode(self) -> Self::NetworkData {
        net_data::BlockId::from_bytes(self.as_bytes())
    }
}

impl Encode for Checkpoints {
    type NetworkData = net_data::BlockIds;

    fn encode(self) -> Self::NetworkData {
        net_data::BlockIds::from_bytes(self.as_slice())
    }
}

impl Encode for Vec<key::Hash> {
    type NetworkData = net_data::BlockIds;

    fn encode(self) -> Self::NetworkData {
        block::try_ids_from_iter(self.iter()).unwrap()
    }
}
use crate::{
    blockcfg::{Block, Fragment, Header, HeaderId},
    intercom,
    topology::{Gossip, Gossips, NodeId},
};
use chain_core::{
    packer::Codec,
    property::{DeserializeFromSlice, Serialize},
};
use chain_network::{
    data as net_data,
    error::{Code, Error},
};
use futures::stream::{self, TryStreamExt};
use std::convert::TryFrom;

fn read<T, U>(src: &T) -> Result<U, Error>
where
    T: AsRef<[u8]>,
    U: DeserializeFromSlice,
{
    U::deserialize_from_slice(&mut Codec::new(src.as_ref()))
        .map_err(|e| Error::new(Code::InvalidArgument, e))
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

    fn encode(&self) -> Self::NetworkData;
}

pub type ResponseStream<T> =
    stream::MapOk<intercom::ReplyStream<T, Error>, fn(T) -> <T as Encode>::NetworkData>;

pub fn response_stream<T: Encode>(
    reply_stream: intercom::ReplyStream<T, Error>,
) -> ResponseStream<T> {
    reply_stream.map_ok(|item| item.encode())
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

impl Decode for net_data::Header {
    type Object = Header;

    fn decode(self) -> Result<Self::Object, Error> {
        read(&self)
    }
}

impl Decode for net_data::Fragment {
    type Object = Fragment;

    fn decode(self) -> Result<Self::Object, Error> {
        Fragment::deserialize_from_slice(&mut Codec::new(self.as_bytes()))
            .map_err(|e| Error::new(Code::InvalidArgument, e))
    }
}

impl Decode for net_data::gossip::Node {
    type Object = Gossip;
    fn decode(self) -> Result<Self::Object, Error> {
        Gossip::deserialize_from_slice(&mut Codec::new(self.as_bytes()))
            .map_err(|e| Error::new(Code::InvalidArgument, e))
    }
}

impl Decode for net_data::NodeId {
    type Object = NodeId;
    fn decode(self) -> Result<Self::Object, Error> {
        NodeId::try_from(self.as_bytes()).map_err(|e| Error::new(Code::InvalidArgument, e))
    }
}

impl<T, N> Encode for Vec<T>
where
    T: Encode<NetworkData = N>,
{
    type NetworkData = Box<[N]>;

    fn encode(&self) -> Box<[N]> {
        self.iter().map(Encode::encode).collect::<Vec<N>>().into()
    }
}

impl Encode for HeaderId {
    type NetworkData = net_data::BlockId;

    fn encode(&self) -> Self::NetworkData {
        net_data::BlockId::try_from(self.as_bytes()).unwrap()
    }
}

impl Encode for Block {
    type NetworkData = net_data::Block;

    fn encode(&self) -> Self::NetworkData {
        net_data::Block::from_bytes(self.serialize_as_vec().unwrap())
    }
}

impl Encode for Header {
    type NetworkData = net_data::Header;

    fn encode(&self) -> Self::NetworkData {
        net_data::Header::from_bytes(self.serialize_as_vec().unwrap())
    }
}

impl Encode for Fragment {
    type NetworkData = net_data::Fragment;

    fn encode(&self) -> Self::NetworkData {
        let serialized = self.serialize_as_vec().unwrap();
        net_data::Fragment::from_bytes(serialized)
    }
}

impl Encode for Gossip {
    type NetworkData = net_data::gossip::Node;

    fn encode(&self) -> Self::NetworkData {
        let bytes = self.serialize_as_vec().unwrap();
        net_data::gossip::Node::from_bytes(bytes)
    }
}

impl Encode for Gossips {
    type NetworkData = net_data::gossip::Gossip;

    fn encode(&self) -> Self::NetworkData {
        let nodes = self
            .0
            .iter()
            .map(Gossip::encode)
            .collect::<Vec<net_data::gossip::Node>>()
            .into_boxed_slice();
        net_data::gossip::Gossip { nodes }
    }
}

impl Encode for NodeId {
    type NetworkData = net_data::NodeId;

    fn encode(&self) -> Self::NetworkData {
        net_data::NodeId::try_from(self.as_ref().as_ref()).unwrap()
    }
}

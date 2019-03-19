use crate::gen;

use chain_core::property;
use network_core::{
    client as core_client,
    gossip::{self, Gossip},
};

pub trait FromProtobuf<R>: Sized {
    fn from_message(message: R) -> Result<Self, core_client::Error>;
}

pub trait IntoProtobuf<R> {
    fn into_message(self) -> Result<R, tower_grpc::Status>;
}

pub fn deserialize_bytes<T>(mut buf: &[u8]) -> Result<T, core_client::Error>
where
    T: property::Deserialize,
{
    T::deserialize(&mut buf).map_err(|e| core_client::Error::new(core_client::ErrorKind::Format, e))
}

pub fn deserialize_vec<T>(pb: &[Vec<u8>]) -> Result<Vec<T>, core_client::Error>
where
    T: property::Deserialize,
{
    pb.iter().map(|v| deserialize_bytes(&v[..])).collect()
}

impl<H> FromProtobuf<gen::node::TipResponse> for H
where
    H: property::Header + property::Deserialize,
{
    fn from_message(msg: gen::node::TipResponse) -> Result<Self, core_client::Error> {
        let block_header = deserialize_bytes(&msg.block_header)?;
        Ok(block_header)
    }
}

impl<T> FromProtobuf<gen::node::Block> for T
where
    T: property::Block + property::Deserialize,
{
    fn from_message(msg: gen::node::Block) -> Result<T, core_client::Error> {
        let block = deserialize_bytes(&msg.content)?;
        Ok(block)
    }
}

impl<T> FromProtobuf<gen::node::Header> for T
where
    T: property::Header + property::Deserialize,
{
    fn from_message(msg: gen::node::Header) -> Result<T, core_client::Error> {
        let block = deserialize_bytes(&msg.content)?;
        Ok(block)
    }
}

impl<T> FromProtobuf<gen::node::GossipMessage> for (gossip::NodeId, T)
where
    T: Gossip,
{
    fn from_message(
        msg: gen::node::GossipMessage,
    ) -> Result<(gossip::NodeId, T), core_client::Error> {
        let node_id = match msg.node_id {
            None => Err(core_client::Error::new(
                core_client::ErrorKind::Format,
                "incorrect node encoding",
            )),
            Some(gen::node::gossip_message::NodeId { content }) => {
                match gossip::NodeId::from_slice(&content) {
                    Ok(node_id) => Ok(node_id),
                    Err(_v) => Err(core_client::Error::new(
                        core_client::ErrorKind::Format,
                        "incorrect node encoding",
                    )),
                }
            }
        }?;
        let gossip = deserialize_bytes(&msg.content)?;
        Ok((node_id, gossip))
    }
}

pub fn serialize_to_bytes<T>(obj: &T) -> Result<Vec<u8>, tower_grpc::Status>
where
    T: property::Serialize,
{
    let mut bytes = Vec::new();
    match obj.serialize(&mut bytes) {
        Ok(()) => Ok(bytes),
        Err(_e) => {
            // FIXME: log the error
            let status = tower_grpc::Status::new(
                tower_grpc::Code::InvalidArgument,
                "response serialization failed",
            );
            Err(status)
        }
    }
}

pub fn serialize_to_vec<T>(values: &[T]) -> Result<Vec<Vec<u8>>, tower_grpc::Status>
where
    T: property::Serialize,
{
    values
        .iter()
        .map(serialize_to_bytes)
        .collect()
}

impl<H> IntoProtobuf<gen::node::TipResponse> for H
where
    H: property::Header,
{
    fn into_message(self) -> Result<gen::node::TipResponse, tower_grpc::Status> {
        let block_header = serialize_to_bytes(&self)?;
        Ok(gen::node::TipResponse { block_header })
    }
}

impl<B> IntoProtobuf<gen::node::Block> for B
where
    B: property::Block + property::Serialize,
{
    fn into_message(self) -> Result<gen::node::Block, tower_grpc::Status> {
        let content = serialize_to_bytes(&self)?;
        Ok(gen::node::Block { content })
    }
}

impl<H> IntoProtobuf<gen::node::Header> for H
where
    H: property::Header + property::Serialize,
{
    fn into_message(self) -> Result<gen::node::Header, tower_grpc::Status> {
        let content = serialize_to_bytes(&self)?;
        Ok(gen::node::Header { content })
    }
}

impl<T> IntoProtobuf<gen::node::Transaction> for T
where
    T: property::Transaction + property::Serialize,
{
    fn into_message(self) -> Result<gen::node::Transaction, tower_grpc::Status> {
        let content = serialize_to_bytes(&self)?;
        Ok(gen::node::Transaction { content })
    }
}

impl<G> IntoProtobuf<gen::node::GossipMessage> for (gossip::NodeId, G)
where
    G: Gossip + property::Serialize,
{
    fn into_message(self) -> Result<gen::node::GossipMessage, tower_grpc::Status> {
        let content = self.0.to_bytes();
        let node_id = Some(gen::node::gossip_message::NodeId { content });
        let content = serialize_to_bytes(&self.1)?;
        Ok(gen::node::GossipMessage { node_id, content })
    }
}

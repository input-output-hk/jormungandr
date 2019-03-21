use crate::gen;

use chain_core::property;
use network_core::{
    error as core_error,
    gossip::{self, Gossip},
};

use tower_grpc::{Code, Status};

pub fn error_into_grpc(err: core_error::Error) -> Status {
    use core_error::Code::*;

    let code = match err.code() {
        Canceled => Code::Cancelled,
        Unknown => Code::Unknown,
        InvalidArgument => Code::InvalidArgument,
        NotFound => Code::NotFound,
        Unimplemented => Code::Unimplemented,
        Internal => Code::Internal,
        // When a new case has to be added here, remember to
        // add the corresponding case in error_from_grpc below.
    };

    Status::new(code, format!("{}", err))
}

pub fn error_from_grpc(e: Status) -> core_error::Error {
    use tower_grpc::Code::*;

    let code = match e.code() {
        Cancelled => core_error::Code::Canceled,
        Unknown => core_error::Code::Unknown,
        InvalidArgument => core_error::Code::InvalidArgument,
        NotFound => core_error::Code::NotFound,
        Unimplemented => core_error::Code::Unimplemented,
        Internal => core_error::Code::Internal,
        _ => core_error::Code::Unknown,
    };

    core_error::Error::new(code, e)
}

pub trait FromProtobuf<R>: Sized {
    fn from_message(message: R) -> Result<Self, core_error::Error>;
}

pub trait IntoProtobuf<R> {
    fn into_message(self) -> Result<R, tower_grpc::Status>;
}

pub fn deserialize_bytes<T>(mut buf: &[u8]) -> Result<T, core_error::Error>
where
    T: property::Deserialize,
{
    T::deserialize(&mut buf)
        .map_err(|e| core_error::Error::new(core_error::Code::InvalidArgument, e))
}

pub fn deserialize_vec<T>(pb: &[Vec<u8>]) -> Result<Vec<T>, core_error::Error>
where
    T: property::Deserialize,
{
    pb.iter().map(|v| deserialize_bytes(&v[..])).collect()
}

impl<H> FromProtobuf<gen::node::TipResponse> for H
where
    H: property::Header + property::Deserialize,
{
    fn from_message(msg: gen::node::TipResponse) -> Result<Self, core_error::Error> {
        let block_header = deserialize_bytes(&msg.block_header)?;
        Ok(block_header)
    }
}

impl<T> FromProtobuf<gen::node::Block> for T
where
    T: property::Block + property::Deserialize,
{
    fn from_message(msg: gen::node::Block) -> Result<T, core_error::Error> {
        let block = deserialize_bytes(&msg.content)?;
        Ok(block)
    }
}

impl<T> FromProtobuf<gen::node::Header> for T
where
    T: property::Header + property::Deserialize,
{
    fn from_message(msg: gen::node::Header) -> Result<T, core_error::Error> {
        let header = deserialize_bytes(&msg.content)?;
        Ok(header)
    }
}

impl<T> FromProtobuf<gen::node::Transaction> for T
where
    T: property::Transaction + property::Deserialize,
{
    fn from_message(msg: gen::node::Transaction) -> Result<T, core_error::Error> {
        let tx = deserialize_bytes(&msg.content)?;
        Ok(tx)
    }
}

impl<T> FromProtobuf<gen::node::GossipMessage> for (gossip::NodeId, T)
where
    T: Gossip,
{
    fn from_message(
        msg: gen::node::GossipMessage,
    ) -> Result<(gossip::NodeId, T), core_error::Error> {
        let node_id = match msg.node_id {
            None => Err(core_error::Error::new(
                core_error::Code::InvalidArgument,
                "incorrect node encoding",
            )),
            Some(gen::node::gossip_message::NodeId { content }) => {
                match gossip::NodeId::from_slice(&content) {
                    Ok(node_id) => Ok(node_id),
                    Err(_v) => Err(core_error::Error::new(
                        core_error::Code::InvalidArgument,
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
    values.iter().map(serialize_to_bytes).collect()
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

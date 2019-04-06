use crate::gen;

use chain_core::property;
use network_core::{
    error as core_error,
    gossip::{Gossip, Node},
};

use tower_grpc::{Code, Status};

pub fn error_into_grpc(err: core_error::Error) -> Status {
    use core_error::Code::*;

    let code = match err.code() {
        Canceled => Code::Cancelled,
        Unknown => Code::Unknown,
        InvalidArgument => Code::InvalidArgument,
        NotFound => Code::NotFound,
        FailedPrecondition => Code::FailedPrecondition,
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
        FailedPrecondition => core_error::Code::FailedPrecondition,
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

impl<T> FromProtobuf<gen::node::Message> for T
where
    T: property::Message + property::Deserialize,
{
    fn from_message(msg: gen::node::Message) -> Result<T, core_error::Error> {
        let tx = deserialize_bytes(&msg.content)?;
        Ok(tx)
    }
}

impl<T> FromProtobuf<gen::node::Gossip> for Gossip<T>
where
    T: Node,
{
    fn from_message(msg: gen::node::Gossip) -> Result<Gossip<T>, core_error::Error> {
        let nodes = deserialize_vec(&msg.nodes)?;
        let gossip = Gossip::from_nodes(nodes);
        Ok(gossip)
    }
}

pub fn serialize_to_bytes<T>(obj: &T) -> Result<Vec<u8>, Status>
where
    T: property::Serialize,
{
    let mut bytes = Vec::new();
    match obj.serialize(&mut bytes) {
        Ok(()) => Ok(bytes),
        Err(_e) => {
            // TODO: log the error
            let status = Status::new(Code::Internal, "response serialization failed");
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

impl<T> IntoProtobuf<gen::node::Message> for T
where
    T: property::Message + property::Serialize,
{
    fn into_message(self) -> Result<gen::node::Message, tower_grpc::Status> {
        let content = serialize_to_bytes(&self)?;
        Ok(gen::node::Message { content })
    }
}

impl<T> IntoProtobuf<gen::node::Gossip> for Gossip<T>
where
    T: Node,
{
    fn into_message(self) -> Result<gen::node::Gossip, tower_grpc::Status> {
        let nodes = serialize_to_vec(self.nodes())?;
        Ok(gen::node::Gossip { nodes })
    }
}

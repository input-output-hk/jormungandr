use crate::gen;

use chain_core::{
    mempack::{self, ReadBuf},
    property,
};
use network_core::error as core_error;
use network_core::gossip::{Gossip, Node, NodeId};
use network_core::subscription::{BlockEvent, ChainPullRequest};

use tower_grpc::{
    metadata::{BinaryMetadataValue, MetadataMap},
    Code, Status,
};

// Name of the binary metadata key used to pass the node ID in subscription requests.
const NODE_ID_HEADER: &'static str = "node-id-bin";

pub fn error_into_grpc(err: core_error::Error) -> Status {
    use core_error::Code::*;

    let code = match err.code() {
        Canceled => Code::Cancelled,
        Unknown => Code::Unknown,
        InvalidArgument => Code::InvalidArgument,
        NotFound => Code::NotFound,
        FailedPrecondition => Code::FailedPrecondition,
        Aborted => Code::Aborted,
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
        Aborted => core_error::Code::Aborted,
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

pub fn deserialize_repeated_bytes<T>(pb: &[Vec<u8>]) -> Result<Vec<T>, core_error::Error>
where
    T: property::Deserialize,
{
    pb.iter().map(|v| deserialize_bytes(&v[..])).collect()
}

pub fn parse_bytes<T>(buf: &[u8]) -> Result<T, core_error::Error>
where
    T: mempack::Readable,
{
    let mut buf = ReadBuf::from(buf);
    T::read(&mut buf).map_err(|e| core_error::Error::new(core_error::Code::InvalidArgument, e))
}

pub fn parse_repeated_bytes<T>(pb: &[Vec<u8>]) -> Result<Vec<T>, core_error::Error>
where
    T: mempack::Readable,
{
    pb.iter().map(|v| parse_bytes(&v[..])).collect()
}

impl<H> FromProtobuf<gen::node::TipResponse> for H
where
    H: property::Header + mempack::Readable,
{
    fn from_message(msg: gen::node::TipResponse) -> Result<Self, core_error::Error> {
        let block_header = parse_bytes(&msg.block_header)?;
        Ok(block_header)
    }
}

impl<T> FromProtobuf<gen::node::Block> for T
where
    T: property::Block + mempack::Readable,
{
    fn from_message(msg: gen::node::Block) -> Result<T, core_error::Error> {
        let block = deserialize_bytes(&msg.content)?;
        Ok(block)
    }
}

impl<T> FromProtobuf<gen::node::Header> for T
where
    T: property::Header + mempack::Readable,
{
    fn from_message(msg: gen::node::Header) -> Result<T, core_error::Error> {
        let header = parse_bytes(&msg.content)?;
        Ok(header)
    }
}

impl<Id> FromProtobuf<gen::node::PullHeadersRequest> for ChainPullRequest<Id>
where
    Id: property::BlockId + mempack::Readable,
{
    fn from_message(msg: gen::node::PullHeadersRequest) -> Result<Self, core_error::Error> {
        let from = parse_repeated_bytes(&msg.from)?;
        let to = parse_bytes(&msg.to)?;
        Ok(ChainPullRequest { from, to })
    }
}

impl<T> FromProtobuf<gen::node::BlockEvent> for BlockEvent<T>
where
    T: property::Block + property::HasHeader,
    T::Header: mempack::Readable,
    T::Id: mempack::Readable,
{
    fn from_message(msg: gen::node::BlockEvent) -> Result<Self, core_error::Error> {
        use gen::node::block_event::*;

        let event = match msg.item {
            Some(Item::Announce(header)) => {
                let header = parse_bytes(&header.content)?;
                BlockEvent::Announce(header)
            }
            Some(Item::Solicit(ids)) => {
                let ids = parse_repeated_bytes(&ids.ids)?;
                BlockEvent::Solicit(ids)
            }
            Some(Item::Missing(req)) => {
                let req = ChainPullRequest::from_message(req)?;
                BlockEvent::Missing(req)
            }
            None => {
                return Err(core_error::Error::new(
                    core_error::Code::InvalidArgument,
                    "invalid BlockEvent payload, one of the fields is required",
                ))
            }
        };
        Ok(event)
    }
}

impl<T> FromProtobuf<gen::node::Message> for T
where
    T: property::Message + mempack::Readable,
{
    fn from_message(msg: gen::node::Message) -> Result<T, core_error::Error> {
        let tx = deserialize_bytes(&msg.content)?;
        Ok(tx)
    }
}

impl<T> FromProtobuf<gen::node::Gossip> for Gossip<T>
where
    T: Node + property::Deserialize,
{
    fn from_message(msg: gen::node::Gossip) -> Result<Gossip<T>, core_error::Error> {
        let mut nodes = Vec::with_capacity(msg.nodes.len());
        for proto_node in msg.nodes {
            let node = T::deserialize(&proto_node[..])
                .map_err(|e| core_error::Error::new(core_error::Code::InvalidArgument, e))?;
            nodes.push(node);
        }
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

pub fn serialize_to_repeated_bytes<T>(values: &[T]) -> Result<Vec<Vec<u8>>, tower_grpc::Status>
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

impl IntoProtobuf<gen::node::PushHeadersResponse> for () {
    fn into_message(self) -> Result<gen::node::PushHeadersResponse, tower_grpc::Status> {
        Ok(gen::node::PushHeadersResponse {})
    }
}

impl IntoProtobuf<gen::node::UploadBlocksResponse> for () {
    fn into_message(self) -> Result<gen::node::UploadBlocksResponse, tower_grpc::Status> {
        Ok(gen::node::UploadBlocksResponse {})
    }
}

impl<Id> IntoProtobuf<gen::node::PullHeadersRequest> for ChainPullRequest<Id>
where
    Id: property::BlockId + property::Serialize,
{
    fn into_message(self) -> Result<gen::node::PullHeadersRequest, tower_grpc::Status> {
        let from = serialize_to_repeated_bytes(&self.from)?;
        let to = serialize_to_bytes(&self.to)?;
        Ok(gen::node::PullHeadersRequest { from, to })
    }
}

impl<T> IntoProtobuf<gen::node::BlockEvent> for BlockEvent<T>
where
    T: property::Block + property::HasHeader,
    T::Header: property::Serialize,
{
    fn into_message(self) -> Result<gen::node::BlockEvent, tower_grpc::Status> {
        use gen::node::block_event::*;

        let item = match self {
            BlockEvent::Announce(header) => {
                let content = serialize_to_bytes(&header)?;
                Item::Announce(gen::node::Header { content })
            }
            BlockEvent::Solicit(ids) => {
                let ids = serialize_to_repeated_bytes(&ids)?;
                Item::Solicit(gen::node::BlockIds { ids })
            }
            BlockEvent::Missing(req) => {
                let req = req.into_message()?;
                Item::Missing(req)
            }
        };
        Ok(gen::node::BlockEvent { item: Some(item) })
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
    T: Node + property::Serialize,
{
    fn into_message(self) -> Result<gen::node::Gossip, tower_grpc::Status> {
        let nodes = serialize_to_repeated_bytes(self.nodes())?;
        Ok(gen::node::Gossip { nodes })
    }
}

pub fn decode_node_id<Id>(metadata: &MetadataMap) -> Result<Id, core_error::Error>
where
    Id: NodeId + property::Deserialize,
{
    match metadata.get_bin(NODE_ID_HEADER) {
        None => Err(core_error::Error::new(
            core_error::Code::InvalidArgument,
            format!("missing metadata {}", NODE_ID_HEADER),
        )),
        Some(val) => {
            let val = val.to_bytes().map_err(|e| {
                core_error::Error::new(
                    core_error::Code::InvalidArgument,
                    format!("invalid metadata value {}: {}", NODE_ID_HEADER, e),
                )
            })?;
            let id = deserialize_bytes(&val).map_err(|e| {
                core_error::Error::new(
                    core_error::Code::InvalidArgument,
                    format!("invalid node ID in {}: {}", NODE_ID_HEADER, e),
                )
            })?;
            Ok(id)
        }
    }
}

pub fn encode_node_id<Id>(id: &Id, metadata: &mut MetadataMap) -> Result<(), Status>
where
    Id: NodeId + property::Serialize,
{
    let bytes = serialize_to_bytes(id)?;
    let val = BinaryMetadataValue::from_bytes(&bytes);
    metadata.insert_bin(NODE_ID_HEADER, val);
    Ok(())
}

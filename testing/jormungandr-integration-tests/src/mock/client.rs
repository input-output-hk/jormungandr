use crate::mock::{
    proto::{
        node::{
            Block, BlockIds, Fragment, FragmentIds, HandshakeRequest, HandshakeResponse, Header,
            PullBlocksToTipRequest, PullHeadersRequest, PushHeadersResponse, TipRequest,
            UploadBlocksResponse,
        },
        node_grpc::NodeClient,
    },
    read_into,
};
use chain_core::property::FromStr;
use chain_core::property::Serialize;
use chain_impl_mockchain as chain;
use chain_impl_mockchain::{
    block::{Block as ChainBlock, Header as ChainHeader},
    key::Hash,
};

use futures::Future;
use grpc::{ClientRequestSink, ClientStubExt, SingleResponse};
use protobuf::RepeatedField;

pub use futures::executor::block_on;

#[macro_export]
macro_rules! response_to_vec {
    ( $e:expr ) => {{
        $crate::mock::client::block_on($e.into_future().drop_metadata())
            .unwrap()
            .into_iter()
            .map(|x| $crate::mock::read_into(x.get_content()))
            .collect()
    }};
}

#[macro_export]
macro_rules! response_to_err {
    ( $e:expr ) => {{
        $crate::mock::client::block_on($e.into_future().drop_metadata())
            .err()
            .expect("response is not an error")
    }};
}

error_chain! {
    errors {
        InvalidRequest (grpc_error: grpc::Error) {
            display("request failed with message {}", grpc_error),
        }

        InvalidAddressFormat (address: String) {
            display("could not parse address '{}'. HINT: accepted format example: /ip4/127.0.0.1/tcp/9000", address),
        }

    }
}

fn push_one<T, R>(
    req: impl Future<Output = grpc::Result<(ClientRequestSink<T>, SingleResponse<R>)>>,
    item: T,
) -> Result<R>
where
    T: Send + 'static,
    R: Send + 'static,
{
    let (mut sink, resp) = block_on(req).unwrap();
    block_on(sink.wait()).unwrap();
    sink.send_data(item).unwrap();
    sink.finish().unwrap();
    block_on(resp.drop_metadata()).map_err(|err| ErrorKind::InvalidRequest(err).into())
}

pub struct JormungandrClient {
    client: NodeClient,
    host: String,
    port: u16,
}

impl Clone for JormungandrClient {
    fn clone(&self) -> Self {
        JormungandrClient::new(&self.host, self.port)
    }
}

impl JormungandrClient {
    pub fn from_address(address: &str) -> Result<Self> {
        let elements: Vec<&str> = address.split("/").collect();

        let host = elements.get(2);
        let port = elements.get(4);

        if host.is_none() || port.is_none() {
            return Err(ErrorKind::InvalidAddressFormat(address.to_owned()).into());
        }

        let port: u16 = port
            .unwrap()
            .parse()
            .map_err(|_err| ErrorKind::InvalidAddressFormat(address.to_owned()))?;
        Ok(Self::new(host.unwrap(), port))
    }

    pub fn new(host: &str, port: u16) -> Self {
        let client_conf = Default::default();
        let client = NodeClient::new_plain(host, port, client_conf).unwrap();
        Self {
            client: client,
            host: host.to_owned(),
            port: port,
        }
    }

    pub fn get_blocks(&self, blocks_id: &Vec<Hash>) -> grpc::StreamingResponse<Block> {
        let block_ids_u8 = blocks_id
            .iter()
            .map(|x| x.as_ref().iter().cloned().collect())
            .collect::<Vec<Vec<u8>>>();
        let mut block_id = BlockIds::new();
        block_id.set_ids(RepeatedField::from_vec(block_ids_u8));
        self.client
            .get_blocks(grpc::RequestOptions::new(), block_id)
    }

    pub fn get_headers(&self, blocks_id: &Vec<Hash>) -> grpc::StreamingResponse<Header> {
        let block_ids_u8 = blocks_id
            .iter()
            .map(|x| x.as_ref().iter().cloned().collect())
            .collect::<Vec<Vec<u8>>>();
        let mut block_id = BlockIds::new();
        block_id.set_ids(RepeatedField::from_vec(block_ids_u8));
        self.client
            .get_headers(grpc::RequestOptions::new(), block_id)
    }

    pub fn handshake(&self) -> HandshakeResponse {
        let resp = self
            .client
            .handshake(grpc::RequestOptions::new(), HandshakeRequest::new());
        block_on(resp.drop_metadata()).unwrap()
    }

    pub fn get_genesis_block_hash(&self) -> Hash {
        Hash::from_str(&hex::encode(self.handshake().block0)).unwrap()
    }

    pub fn get_tip(&self) -> ChainHeader {
        let resp = self
            .client
            .tip(grpc::RequestOptions::new(), TipRequest::new());
        let tip = block_on(resp.drop_metadata()).unwrap();
        read_into(&tip.get_block_header())
    }

    pub fn upload_blocks(&self, chain_block: ChainBlock) -> Result<UploadBlocksResponse> {
        let mut bytes = Vec::with_capacity(4096);
        chain_block.serialize(&mut bytes).unwrap();
        let mut block = Block::new();
        block.set_content(bytes);
        let req = self.client.upload_blocks(grpc::RequestOptions::new());
        push_one(req, block)
    }

    pub fn pull_blocks_to_tip(&self, from: Hash) -> grpc::StreamingResponse<Block> {
        let mut request = PullBlocksToTipRequest::new();
        request.set_from(RepeatedField::from_vec(vec![from.as_ref().to_vec()]));

        self.client
            .pull_blocks_to_tip(grpc::RequestOptions::new(), request)
    }

    pub fn pull_headers(
        &self,
        from: Option<Hash>,
        to: Option<Hash>,
    ) -> grpc::StreamingResponse<Header> {
        let mut request = PullHeadersRequest::new();
        if let Some(hash) = to {
            request.set_to(hash.as_ref().to_vec());
        }
        if let Some(hash) = from {
            request.set_from(RepeatedField::from_vec(vec![hash.as_ref().to_vec()]));
        }

        self.client
            .pull_headers(grpc::RequestOptions::new(), request)
    }

    pub fn push_header(&self, chain_header: chain::block::Header) -> Result<PushHeadersResponse> {
        let mut header = Header::new();
        header.set_content(chain_header.serialize_as_vec().unwrap());
        let req = self.client.push_headers(grpc::RequestOptions::new());
        push_one(req, header)
    }

    pub fn get_fragments(&self, ids: Vec<Hash>) -> grpc::StreamingResponse<Fragment> {
        let mut fragment_ids = FragmentIds::new();
        let encoded_hashes: Vec<Vec<u8>> = ids.iter().map(|hash| hash.as_ref().to_vec()).collect();
        fragment_ids.set_ids(RepeatedField::from_vec(encoded_hashes));
        self.client
            .get_fragments(grpc::RequestOptions::new(), fragment_ids)
    }
}

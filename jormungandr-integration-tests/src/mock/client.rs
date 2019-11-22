extern crate base64;
extern crate chain_impl_mockchain;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate hex;
extern crate protobuf;

use crate::mock::{
    grpc::{ClientStubExt, Metadata},
    proto::{
        node::{
            Block, BlockIds, Fragment, FragmentIds, HandshakeRequest, HandshakeResponse, Header,
            PullBlocksToTipRequest, PullHeadersRequest, PushHeadersResponse, TipRequest,
            UploadBlocksResponse,
        },
        node_grpc::{Node, NodeClient},
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
use protobuf::RepeatedField;

#[macro_export]
macro_rules! response_to_vec {
    ( $e:expr ) => {{
        $e.into_future()
            .wait_drop_metadata()
            .unwrap()
            .into_iter()
            .map(|x| read_into(x.get_content()))
            .collect()
    }};
}

#[macro_export]
macro_rules! response_to_err {
    ( $e:expr ) => {{
        $e.into_future()
            .wait_drop_metadata()
            .err()
            .expect("response is not an error")
    }};
}

error_chain! {
    errors {
        InvalidRequest (message: String) {
            display("request failed with message {}", message),
        }

        InvalidAddressFormat (address: String) {
            display("could not parse address '{}'. HINT: accepted format example: /ip4/127.0.0.1/tcp/9000", address),
        }

    }
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
        let (_, handshake_resp, _) = resp.wait().unwrap();
        handshake_resp
    }

    pub fn get_genesis_block_hash(&self) -> Hash {
        Hash::from_str(&hex::encode(self.handshake().block0)).unwrap()
    }

    pub fn get_tip(&self) -> ChainHeader {
        let resp = self
            .client
            .tip(grpc::RequestOptions::new(), TipRequest::new());
        let (_, tip, _) = resp.wait().unwrap();
        read_into(&tip.get_block_header())
    }

    pub fn upload_blocks(
        &self,
        chain_block: ChainBlock,
    ) -> Result<(Metadata, UploadBlocksResponse, Metadata)> {
        let mut bytes = Vec::with_capacity(4096);
        chain_block.serialize(&mut bytes).unwrap();
        let mut block = Block::new();
        block.set_content(bytes);
        let resp = self.client.upload_blocks(
            grpc::RequestOptions::new(),
            grpc::StreamingRequest::single(block),
        );
        resp.wait()
            .map_err(|err| ErrorKind::InvalidRequest(err.to_string()).into())
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

    pub fn push_header(
        &self,
        chain_header: chain::block::Header,
    ) -> Result<(Metadata, PushHeadersResponse, Metadata)> {
        let mut header = Header::new();
        header.set_content(chain_header.serialize_as_vec().unwrap());
        let resp = self.client.push_headers(
            grpc::RequestOptions::new(),
            grpc::StreamingRequest::single(header),
        );
        resp.wait()
            .map_err(|err| ErrorKind::InvalidRequest(err.to_string()).into())
    }

    pub fn get_fragments(&self, ids: Vec<Hash>) -> grpc::StreamingResponse<Fragment> {
        let mut fragment_ids = FragmentIds::new();
        let encoded_hashes: Vec<Vec<u8>> = ids.iter().map(|hash| hash.as_ref().to_vec()).collect();
        fragment_ids.set_ids(RepeatedField::from_vec(encoded_hashes));
        self.client
            .get_fragments(grpc::RequestOptions::new(), fragment_ids)
    }
}

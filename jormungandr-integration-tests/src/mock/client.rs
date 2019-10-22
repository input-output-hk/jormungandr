extern crate base64;
extern crate chain_impl_mockchain;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate hex;
extern crate protobuf;

use crate::mock::{
    grpc::*,
    proto::{node::*, node_grpc::*},
    read_into,
};
use chain_core::property::FromStr;
use chain_core::property::Serialize;
use chain_impl_mockchain as chain;
use chain_impl_mockchain::{
    block::{Block as ChainBlock, Header as ChainHeader},
    fragment::Fragment as ChainFragment,
    key::Hash,
};
use protobuf::RepeatedField;

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
    pub fn new(host: &str, port: u16) -> Self {
        let client_conf = Default::default();
        let client = NodeClient::new_plain(host, port, client_conf).unwrap();
        Self {
            client: client,
            host: host.to_owned(),
            port: port,
        }
    }

    pub fn get_blocks(&self, blocks_id: &Vec<Hash>) -> Vec<ChainBlock> {
        self.get_blocks_stream(blocks_id)
            .into_future()
            .wait_drop_metadata()
            .unwrap()
            .into_iter()
            .map(|x| read_into(x.get_content()))
            .collect()
    }

    fn get_blocks_stream(&self, blocks_id: &Vec<Hash>) -> grpc::StreamingResponse<Block> {
        let block_ids_u8 = blocks_id
            .iter()
            .map(|x| x.as_ref().iter().cloned().collect())
            .collect::<Vec<Vec<u8>>>();
        let mut block_id = BlockIds::new();
        block_id.set_ids(RepeatedField::from_vec(block_ids_u8));
        self.client
            .get_blocks(grpc::RequestOptions::new(), block_id)
    }

    pub fn get_headers(&self, blocks_id: &Vec<Hash>) -> Vec<ChainHeader> {
        self.get_headers_stream(blocks_id)
            .into_future()
            .wait_drop_metadata()
            .unwrap()
            .into_iter()
            .map(|x| read_into(x.get_content()))
            .collect()
    }

    fn get_headers_stream(&self, blocks_id: &Vec<Hash>) -> grpc::StreamingResponse<Header> {
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

    pub fn upload_blocks_internal(
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
    }

    pub fn upload_blocks(&self, chain_block: ChainBlock) -> UploadBlocksResponse {
        let (_, response, _) = self.upload_blocks_internal(chain_block).unwrap();
        response
    }

    pub fn upload_blocks_err(&self, chain_block: ChainBlock) -> grpc::Error {
        self.upload_blocks_internal(chain_block).err().unwrap()
    }

    pub fn pull_blocks_to_tip(&self, from: Hash) -> Vec<ChainBlock> {
        self.pull_blocks_to_tip_stream(from)
            .into_future()
            .wait_drop_metadata()
            .unwrap()
            .into_iter()
            .map(|x| read_into(x.get_content()))
            .collect()
    }

    pub fn pull_blocks_to_tip_stream(&self, from: Hash) -> grpc::StreamingResponse<Block> {
        let mut request = PullBlocksToTipRequest::new();
        request.set_from(RepeatedField::from_vec(vec![from.as_ref().to_vec()]));

        self.client
            .pull_blocks_to_tip(grpc::RequestOptions::new(), request)
    }

    pub fn pull_headers(&self, from: Option<Hash>, to: Option<Hash>) -> Vec<ChainHeader> {
        self.pull_headers_stream(from, to)
            .into_future()
            .wait_drop_metadata()
            .unwrap()
            .into_iter()
            .map(|x| read_into(x.get_content()))
            .collect()
    }

    pub fn pull_headers_get_err(&self, from: Option<Hash>, to: Option<Hash>) -> grpc::Error {
        self.pull_headers_stream(from, to)
            .into_future()
            .wait_drop_metadata()
            .err()
            .unwrap()
    }

    fn pull_headers_stream(
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

    pub fn push_header(&self, chain_header: chain::block::Header) -> PushHeadersResponse {
        let (_, push_headers_response, _) = self.push_header_internal(chain_header).unwrap();
        push_headers_response
    }

    pub fn push_header_err(&self, chain_header: chain::block::Header) -> grpc::Error {
        self.push_header_internal(chain_header).err().unwrap()
    }

    fn push_header_internal(
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
    }

    pub fn get_fragments(&self, ids: Vec<Hash>) -> Vec<ChainFragment> {
        self.get_fragments_stream(ids)
            .into_future()
            .wait_drop_metadata()
            .unwrap()
            .into_iter()
            .map(|x| read_into(x.get_content()))
            .collect()
    }

    pub fn get_fragments_err(&self, ids: Vec<Hash>) -> grpc::Error {
        self.get_fragments_stream(ids)
            .into_future()
            .wait_drop_metadata()
            .err()
            .unwrap()
    }

    pub fn get_fragments_stream(&self, ids: Vec<Hash>) -> grpc::StreamingResponse<Fragment> {
        let mut fragment_ids = FragmentIds::new();
        let encoded_hashes: Vec<Vec<u8>> = ids.iter().map(|hash| hash.as_ref().to_vec()).collect();
        fragment_ids.set_ids(RepeatedField::from_vec(encoded_hashes));
        self.client
            .get_fragments(grpc::RequestOptions::new(), fragment_ids)
    }
}

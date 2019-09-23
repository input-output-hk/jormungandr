extern crate base64;
extern crate chain_impl_mockchain;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate hex;
extern crate protobuf;

use crate::{grpc::ClientStubExt, node::*, node_grpc::*};
use chain_core::mempack::ReadBuf;
use chain_core::mempack::Readable;
use chain_core::property::Serialize;
use chain_impl_mockchain as chain;
use chain_impl_mockchain::key::Hash;
use protobuf::RepeatedField;

pub struct JormungandrClient {
    client: NodeClient,
}

impl JormungandrClient {
    pub fn new(host: &str, port: u16) -> Self {
        let client_conf = Default::default();
        let client = NodeClient::new_plain(host, port, client_conf).unwrap();
        Self { client }
    }

    pub fn get_blocks(&self, blocks_id: Vec<&Hash>) -> grpc::StreamingResponse<Block> {
        let block_ids_u8 = blocks_id
            .iter()
            .map(|x| x.as_ref().iter().cloned().collect())
            .collect::<Vec<Vec<u8>>>();
        let mut block_id = BlockIds::new();
        block_id.set_ids(RepeatedField::from_vec(block_ids_u8));
        self.client
            .get_blocks(grpc::RequestOptions::new(), block_id)
    }

    pub fn get_headers(&self, blocks_id: Vec<&str>) -> grpc::StreamingResponse<Header> {
        let block_ids_u8 = blocks_id
            .iter()
            .map(|x| hex::decode(x).unwrap())
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

    pub fn get_tip(&self) -> TipResponse {
        let resp = self
            .client
            .tip(grpc::RequestOptions::new(), TipRequest::new());
        let (_, tip, _) = resp.wait().unwrap();
        tip
    }

    pub fn upload_blocks(&self) {
        let block_content = "0002000000000000215900000019000000010e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a86ebcedcaf48791a48ba1dcd59a2b33dbea3e22667f018a7ce9d66a89cfecec97759bd4e856cc185f86ccec81fb391942ecba317cb288be9f2a6e4409bc8c790704961ce2dd438a3fae283ee296275be09b4668e38c84ad11bad733ad99423c64068927c11d62a0541d1fb81b696e0c2045d1ab22907de21dbf491aa557e4a60624f5f5f5d3785a70f48fe03e6cc39f4fd8bb3fb0670f536ccb66741b6640720b000000008e39e438ec4562fa06024c130ae245d57ed62c098ceee61a1c406f6448ef133e80d83100a50294a87eae15ce594d4994d76b61881f119871028432da1f278408ff60e24bdad06e83bfa28984a87df75d1d116b1105b2b04ab77c747012b679825844a2878a7198bc7407d8c852a9d27162133b56b2fd76bb7ecc04f41c5df91fe5d62120ea3a7bc96c6d651fad714f903a7ecfdf0b19a903276c51bdfbb05526b3d782f0534c6d8ffe7083a73be32418ad8e8c6a50c3113e64e2797a962842ad1eefef74e0a0433924198876f024190f0e5fb464269de6ab57b7d725b8be075ae550d6837b34c7546117496caff32046c82ce3ef849f12a4584b086dc8b50623235b4d133ddac2e7e3404c73008111ae023053dbcf79f1909423f422562e0feba656ab9943ca67368ee5af69587a99c68809882e5c048cda5f6fb1614b82dff33618eb412b5d1b0ba353bb00da0f45c61b62f40974f4fceb29ce243ca768754292b5ffb43d8a4deaeeb53c058c1098c48dcaaf6e38181cfeeacf5649875e08639a81225b53bab981fb5a3938fc8b59658bee698ee7aa102c966537b354e559ab27113910458994d483655c2d4fb05b7eb37e92e21d65e1170d76cdda2e8196987796bf7833e81cd42b041e61e0933813964c37dfb2992f9529e7a0a2cfa57ab1";
        let mut block = Block::new();
        block.set_content(hex::decode(block_content).unwrap());
        let resp = self.client.upload_blocks(
            grpc::RequestOptions::new(),
            grpc::StreamingRequest::single(block),
        );
        let (_, upload_block_resp, _) = resp.wait().unwrap();
        println!("item: {:?}", upload_block_resp);
    }

    pub fn pull_blocks_to_tip(&self, from: Hash) -> grpc::StreamingResponse<Block> {
        let mut request = PullBlocksToTipRequest::new();
        request.set_from(RepeatedField::from_vec(vec![from.as_ref().to_vec()]));

        self.client
            .pull_blocks_to_tip(grpc::RequestOptions::new(), request)
    }

    pub fn pull_headers(&self, to: Option<Hash>) -> grpc::StreamingResponse<Header> {
        let mut request = PullHeadersRequest::new();
        if let Some(hash) = to {
            request.set_to(hash.as_ref().to_vec());
        }

        self.client
            .pull_headers(grpc::RequestOptions::new(), request)
    }

    pub fn push_headers(&self, chain_header: chain::block::Header) -> PushHeadersResponse {
        let mut header = Header::new();
        header.set_content(chain_header.serialize_as_vec().unwrap());

        let resp = self.client.push_headers(
            grpc::RequestOptions::new(),
            grpc::StreamingRequest::single(header),
        );
        let (_, push_headers_response, _) = resp.wait().unwrap();
        push_headers_response
    }
}

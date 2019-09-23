extern crate base64;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate hex;
extern crate protobuf;

use crate::{futures::Stream, grpc::SingleResponse, node::*, node_grpc::*};
use chain_impl_mockchain::key::Hash;
use grpc::Server;
use std::thread;

pub fn start(port: u16, genesis_hash: Hash, tip: Hash, version: u32) -> Server {
    let mut server = grpc::ServerBuilder::new_plain();
    server.http.set_port(port);
    server.add_service(NodeServer::new_service_def(JormungandrServerImpl {
        genesis_hash: genesis_hash,
        tip: tip,
        version: version,
    }));

    let server = server.build().expect("server");
    println!("server started on port {}", port);
    server
}

pub struct JormungandrServerImpl {
    genesis_hash: Hash,
    tip: Hash,
    version: u32,
}

impl JormungandrServerImpl {}

impl Node for JormungandrServerImpl {
    fn handshake(
        &self,
        _o: ::grpc::RequestOptions,
        _p: HandshakeRequest,
    ) -> ::grpc::SingleResponse<HandshakeResponse> {
        println!("handshake");
        let mut handshake = HandshakeResponse::new();
        handshake.set_version(self.version);
        handshake.set_block0(self.genesis_hash.as_ref().to_vec());
        SingleResponse::completed(handshake)
    }

    fn tip(
        &self,
        _o: ::grpc::RequestOptions,
        p: TipRequest,
    ) -> ::grpc::SingleResponse<TipResponse> {
        println!("tip");
        let mut tip_response = TipResponse::new();
        tip_response.set_block_header(self.tip.as_ref().to_vec());
        ::grpc::SingleResponse::completed(tip_response)
    }

    fn get_blocks(
        &self,
        _o: ::grpc::RequestOptions,
        p: BlockIds,
    ) -> ::grpc::StreamingResponse<Block> {
        println!("get_blocks");
        ::grpc::StreamingResponse::empty()
    }

    fn get_headers(
        &self,
        _o: ::grpc::RequestOptions,
        p: BlockIds,
    ) -> ::grpc::StreamingResponse<Header> {
        println!("get_headers");
        ::grpc::StreamingResponse::empty()
    }

    fn get_fragments(
        &self,
        _o: ::grpc::RequestOptions,
        p: FragmentIds,
    ) -> ::grpc::StreamingResponse<Fragment> {
        println!("get_fragments");
        ::grpc::StreamingResponse::empty()
    }

    fn pull_headers(
        &self,
        _o: ::grpc::RequestOptions,
        p: PullHeadersRequest,
    ) -> ::grpc::StreamingResponse<Header> {
        println!("pull_headers");
        ::grpc::StreamingResponse::empty()
    }

    fn pull_blocks_to_tip(
        &self,
        _o: ::grpc::RequestOptions,
        p: PullBlocksToTipRequest,
    ) -> ::grpc::StreamingResponse<Block> {
        println!("pull_blocks_to_tip");
        ::grpc::StreamingResponse::empty()
    }

    fn push_headers(
        &self,
        _o: ::grpc::RequestOptions,
        p: ::grpc::StreamingRequest<Header>,
    ) -> ::grpc::SingleResponse<PushHeadersResponse> {
        println!("push_headers");
        let header_response = PushHeadersResponse::new();
        ::grpc::SingleResponse::completed(header_response)
    }

    fn upload_blocks(
        &self,
        _o: ::grpc::RequestOptions,
        p: ::grpc::StreamingRequest<Block>,
    ) -> ::grpc::SingleResponse<UploadBlocksResponse> {
        println!("upload_blocks");
        let block_response = UploadBlocksResponse::new();
        ::grpc::SingleResponse::completed(block_response)
    }

    fn block_subscription(
        &self,
        _o: ::grpc::RequestOptions,
        p: ::grpc::StreamingRequest<Header>,
    ) -> ::grpc::StreamingResponse<BlockEvent> {
        println!("block_subscription");
        ::grpc::StreamingResponse::empty()
    }

    fn content_subscription(
        &self,
        _o: ::grpc::RequestOptions,
        p: ::grpc::StreamingRequest<Fragment>,
    ) -> ::grpc::StreamingResponse<Fragment> {
        println!("content_subscription");
        ::grpc::StreamingResponse::empty()
    }

    fn gossip_subscription(
        &self,
        _o: ::grpc::RequestOptions,
        p: ::grpc::StreamingRequest<Gossip>,
    ) -> ::grpc::StreamingResponse<Gossip> {
        println!("gossip_subscription");
        p.0.map(|x| println!("{:?}", x));
        ::grpc::StreamingResponse::empty()
    }
}

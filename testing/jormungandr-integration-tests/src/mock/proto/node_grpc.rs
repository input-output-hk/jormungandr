// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]


// interface

pub trait Node {
    fn handshake(&self, o: ::grpc::RequestOptions, p: super::node::HandshakeRequest) -> ::grpc::SingleResponse<super::node::HandshakeResponse>;

    fn tip(&self, o: ::grpc::RequestOptions, p: super::node::TipRequest) -> ::grpc::SingleResponse<super::node::TipResponse>;

    fn get_blocks(&self, o: ::grpc::RequestOptions, p: super::node::BlockIds) -> ::grpc::StreamingResponse<super::node::Block>;

    fn get_headers(&self, o: ::grpc::RequestOptions, p: super::node::BlockIds) -> ::grpc::StreamingResponse<super::node::Header>;

    fn get_fragments(&self, o: ::grpc::RequestOptions, p: super::node::FragmentIds) -> ::grpc::StreamingResponse<super::node::Fragment>;

    fn pull_headers(&self, o: ::grpc::RequestOptions, p: super::node::PullHeadersRequest) -> ::grpc::StreamingResponse<super::node::Header>;

    fn pull_blocks_to_tip(&self, o: ::grpc::RequestOptions, p: super::node::PullBlocksToTipRequest) -> ::grpc::StreamingResponse<super::node::Block>;

    fn push_headers(&self, o: ::grpc::RequestOptions, p: ::grpc::StreamingRequest<super::node::Header>) -> ::grpc::SingleResponse<super::node::PushHeadersResponse>;

    fn upload_blocks(&self, o: ::grpc::RequestOptions, p: ::grpc::StreamingRequest<super::node::Block>) -> ::grpc::SingleResponse<super::node::UploadBlocksResponse>;

    fn block_subscription(&self, o: ::grpc::RequestOptions, p: ::grpc::StreamingRequest<super::node::Header>) -> ::grpc::StreamingResponse<super::node::BlockEvent>;

    fn content_subscription(&self, o: ::grpc::RequestOptions, p: ::grpc::StreamingRequest<super::node::Fragment>) -> ::grpc::StreamingResponse<super::node::Fragment>;

    fn gossip_subscription(&self, o: ::grpc::RequestOptions, p: ::grpc::StreamingRequest<super::node::Gossip>) -> ::grpc::StreamingResponse<super::node::Gossip>;
}

// client

pub struct NodeClient {
    grpc_client: ::std::sync::Arc<::grpc::Client>,
    method_Handshake: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::HandshakeRequest, super::node::HandshakeResponse>>,
    method_Tip: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::TipRequest, super::node::TipResponse>>,
    method_GetBlocks: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::BlockIds, super::node::Block>>,
    method_GetHeaders: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::BlockIds, super::node::Header>>,
    method_GetFragments: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::FragmentIds, super::node::Fragment>>,
    method_PullHeaders: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::PullHeadersRequest, super::node::Header>>,
    method_PullBlocksToTip: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::PullBlocksToTipRequest, super::node::Block>>,
    method_PushHeaders: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::Header, super::node::PushHeadersResponse>>,
    method_UploadBlocks: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::Block, super::node::UploadBlocksResponse>>,
    method_BlockSubscription: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::Header, super::node::BlockEvent>>,
    method_ContentSubscription: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::Fragment, super::node::Fragment>>,
    method_GossipSubscription: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::node::Gossip, super::node::Gossip>>,
}

impl ::grpc::ClientStub for NodeClient {
    fn with_client(grpc_client: ::std::sync::Arc<::grpc::Client>) -> Self {
        NodeClient {
            grpc_client: grpc_client,
            method_Handshake: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/Handshake".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Unary,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_Tip: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/Tip".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Unary,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_GetBlocks: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/GetBlocks".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_GetHeaders: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/GetHeaders".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_GetFragments: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/GetFragments".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_PullHeaders: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/PullHeaders".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_PullBlocksToTip: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/PullBlocksToTip".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_PushHeaders: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/PushHeaders".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::ClientStreaming,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_UploadBlocks: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/UploadBlocks".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::ClientStreaming,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_BlockSubscription: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/BlockSubscription".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Bidi,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_ContentSubscription: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/ContentSubscription".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Bidi,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_GossipSubscription: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/iohk.chain.node.Node/GossipSubscription".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Bidi,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
        }
    }
}

impl Node for NodeClient {
    fn handshake(&self, o: ::grpc::RequestOptions, p: super::node::HandshakeRequest) -> ::grpc::SingleResponse<super::node::HandshakeResponse> {
        self.grpc_client.call_unary(o, p, self.method_Handshake.clone())
    }

    fn tip(&self, o: ::grpc::RequestOptions, p: super::node::TipRequest) -> ::grpc::SingleResponse<super::node::TipResponse> {
        self.grpc_client.call_unary(o, p, self.method_Tip.clone())
    }

    fn get_blocks(&self, o: ::grpc::RequestOptions, p: super::node::BlockIds) -> ::grpc::StreamingResponse<super::node::Block> {
        self.grpc_client.call_server_streaming(o, p, self.method_GetBlocks.clone())
    }

    fn get_headers(&self, o: ::grpc::RequestOptions, p: super::node::BlockIds) -> ::grpc::StreamingResponse<super::node::Header> {
        self.grpc_client.call_server_streaming(o, p, self.method_GetHeaders.clone())
    }

    fn get_fragments(&self, o: ::grpc::RequestOptions, p: super::node::FragmentIds) -> ::grpc::StreamingResponse<super::node::Fragment> {
        self.grpc_client.call_server_streaming(o, p, self.method_GetFragments.clone())
    }

    fn pull_headers(&self, o: ::grpc::RequestOptions, p: super::node::PullHeadersRequest) -> ::grpc::StreamingResponse<super::node::Header> {
        self.grpc_client.call_server_streaming(o, p, self.method_PullHeaders.clone())
    }

    fn pull_blocks_to_tip(&self, o: ::grpc::RequestOptions, p: super::node::PullBlocksToTipRequest) -> ::grpc::StreamingResponse<super::node::Block> {
        self.grpc_client.call_server_streaming(o, p, self.method_PullBlocksToTip.clone())
    }

    fn push_headers(&self, o: ::grpc::RequestOptions, p: ::grpc::StreamingRequest<super::node::Header>) -> ::grpc::SingleResponse<super::node::PushHeadersResponse> {
        self.grpc_client.call_client_streaming(o, p, self.method_PushHeaders.clone())
    }

    fn upload_blocks(&self, o: ::grpc::RequestOptions, p: ::grpc::StreamingRequest<super::node::Block>) -> ::grpc::SingleResponse<super::node::UploadBlocksResponse> {
        self.grpc_client.call_client_streaming(o, p, self.method_UploadBlocks.clone())
    }

    fn block_subscription(&self, o: ::grpc::RequestOptions, p: ::grpc::StreamingRequest<super::node::Header>) -> ::grpc::StreamingResponse<super::node::BlockEvent> {
        self.grpc_client.call_bidi(o, p, self.method_BlockSubscription.clone())
    }

    fn content_subscription(&self, o: ::grpc::RequestOptions, p: ::grpc::StreamingRequest<super::node::Fragment>) -> ::grpc::StreamingResponse<super::node::Fragment> {
        self.grpc_client.call_bidi(o, p, self.method_ContentSubscription.clone())
    }

    fn gossip_subscription(&self, o: ::grpc::RequestOptions, p: ::grpc::StreamingRequest<super::node::Gossip>) -> ::grpc::StreamingResponse<super::node::Gossip> {
        self.grpc_client.call_bidi(o, p, self.method_GossipSubscription.clone())
    }
}

// server

pub struct NodeServer;


impl NodeServer {
    pub fn new_service_def<H : Node + 'static + Sync + Send + 'static>(handler: H) -> ::grpc::rt::ServerServiceDefinition {
        let handler_arc = ::std::sync::Arc::new(handler);
        ::grpc::rt::ServerServiceDefinition::new("/iohk.chain.node.Node",
            vec![
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/Handshake".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |o, p| handler_copy.handshake(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/Tip".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |o, p| handler_copy.tip(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/GetBlocks".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |o, p| handler_copy.get_blocks(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/GetHeaders".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |o, p| handler_copy.get_headers(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/GetFragments".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |o, p| handler_copy.get_fragments(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/PullHeaders".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |o, p| handler_copy.pull_headers(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/PullBlocksToTip".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |o, p| handler_copy.pull_blocks_to_tip(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/PushHeaders".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::ClientStreaming,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerClientStreaming::new(move |o, p| handler_copy.push_headers(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/UploadBlocks".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::ClientStreaming,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerClientStreaming::new(move |o, p| handler_copy.upload_blocks(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/BlockSubscription".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::Bidi,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerBidi::new(move |o, p| handler_copy.block_subscription(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/ContentSubscription".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::Bidi,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerBidi::new(move |o, p| handler_copy.content_subscription(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/iohk.chain.node.Node/GossipSubscription".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::Bidi,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerBidi::new(move |o, p| handler_copy.gossip_subscription(o, p))
                    },
                ),
            ],
        )
    }
}

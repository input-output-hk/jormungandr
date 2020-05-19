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


// server interface

pub trait Node {
    fn handshake(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::node::HandshakeRequest>, resp: ::grpc::ServerResponseUnarySink<super::node::HandshakeResponse>) -> ::grpc::Result<()>;

    fn tip(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::node::TipRequest>, resp: ::grpc::ServerResponseUnarySink<super::node::TipResponse>) -> ::grpc::Result<()>;

    fn peers(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::node::PeersRequest>, resp: ::grpc::ServerResponseUnarySink<super::node::PeersResponse>) -> ::grpc::Result<()>;

    fn get_blocks(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::node::BlockIds>, resp: ::grpc::ServerResponseSink<super::node::Block>) -> ::grpc::Result<()>;

    fn get_headers(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::node::BlockIds>, resp: ::grpc::ServerResponseSink<super::node::Header>) -> ::grpc::Result<()>;

    fn get_fragments(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::node::FragmentIds>, resp: ::grpc::ServerResponseSink<super::node::Fragment>) -> ::grpc::Result<()>;

    fn pull_headers(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::node::PullHeadersRequest>, resp: ::grpc::ServerResponseSink<super::node::Header>) -> ::grpc::Result<()>;

    fn pull_blocks_to_tip(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::node::PullBlocksToTipRequest>, resp: ::grpc::ServerResponseSink<super::node::Block>) -> ::grpc::Result<()>;

    fn push_headers(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequest<super::node::Header>, resp: ::grpc::ServerResponseUnarySink<super::node::PushHeadersResponse>) -> ::grpc::Result<()>;

    fn upload_blocks(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequest<super::node::Block>, resp: ::grpc::ServerResponseUnarySink<super::node::UploadBlocksResponse>) -> ::grpc::Result<()>;

    fn block_subscription(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequest<super::node::Header>, resp: ::grpc::ServerResponseSink<super::node::BlockEvent>) -> ::grpc::Result<()>;

    fn fragment_subscription(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequest<super::node::Fragment>, resp: ::grpc::ServerResponseSink<super::node::Fragment>) -> ::grpc::Result<()>;

    fn gossip_subscription(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequest<super::node::Gossip>, resp: ::grpc::ServerResponseSink<super::node::Gossip>) -> ::grpc::Result<()>;
}

// client

pub struct NodeClient {
    grpc_client: ::std::sync::Arc<::grpc::Client>,
}

impl ::grpc::ClientStub for NodeClient {
    fn with_client(grpc_client: ::std::sync::Arc<::grpc::Client>) -> Self {
        NodeClient {
            grpc_client: grpc_client,
        }
    }
}

impl NodeClient {
    pub fn handshake(&self, o: ::grpc::RequestOptions, req: super::node::HandshakeRequest) -> ::grpc::SingleResponse<super::node::HandshakeResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/Handshake"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn tip(&self, o: ::grpc::RequestOptions, req: super::node::TipRequest) -> ::grpc::SingleResponse<super::node::TipResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/Tip"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn peers(&self, o: ::grpc::RequestOptions, req: super::node::PeersRequest) -> ::grpc::SingleResponse<super::node::PeersResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/Peers"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn get_blocks(&self, o: ::grpc::RequestOptions, req: super::node::BlockIds) -> ::grpc::StreamingResponse<super::node::Block> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/GetBlocks"),
            streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_server_streaming(o, req, descriptor)
    }

    pub fn get_headers(&self, o: ::grpc::RequestOptions, req: super::node::BlockIds) -> ::grpc::StreamingResponse<super::node::Header> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/GetHeaders"),
            streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_server_streaming(o, req, descriptor)
    }

    pub fn get_fragments(&self, o: ::grpc::RequestOptions, req: super::node::FragmentIds) -> ::grpc::StreamingResponse<super::node::Fragment> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/GetFragments"),
            streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_server_streaming(o, req, descriptor)
    }

    pub fn pull_headers(&self, o: ::grpc::RequestOptions, req: super::node::PullHeadersRequest) -> ::grpc::StreamingResponse<super::node::Header> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/PullHeaders"),
            streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_server_streaming(o, req, descriptor)
    }

    pub fn pull_blocks_to_tip(&self, o: ::grpc::RequestOptions, req: super::node::PullBlocksToTipRequest) -> ::grpc::StreamingResponse<super::node::Block> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/PullBlocksToTip"),
            streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_server_streaming(o, req, descriptor)
    }

    pub fn push_headers(&self, o: ::grpc::RequestOptions) -> impl ::std::future::Future<Output=::grpc::Result<(::grpc::ClientRequestSink<super::node::Header>, ::grpc::SingleResponse<super::node::PushHeadersResponse>)>> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/PushHeaders"),
            streaming: ::grpc::rt::GrpcStreaming::ClientStreaming,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_client_streaming(o, descriptor)
    }

    pub fn upload_blocks(&self, o: ::grpc::RequestOptions) -> impl ::std::future::Future<Output=::grpc::Result<(::grpc::ClientRequestSink<super::node::Block>, ::grpc::SingleResponse<super::node::UploadBlocksResponse>)>> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/UploadBlocks"),
            streaming: ::grpc::rt::GrpcStreaming::ClientStreaming,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_client_streaming(o, descriptor)
    }

    pub fn block_subscription(&self, o: ::grpc::RequestOptions) -> impl ::std::future::Future<Output=::grpc::Result<(::grpc::ClientRequestSink<super::node::Header>, ::grpc::StreamingResponse<super::node::BlockEvent>)>> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/BlockSubscription"),
            streaming: ::grpc::rt::GrpcStreaming::Bidi,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_bidi(o, descriptor)
    }

    pub fn fragment_subscription(&self, o: ::grpc::RequestOptions) -> impl ::std::future::Future<Output=::grpc::Result<(::grpc::ClientRequestSink<super::node::Fragment>, ::grpc::StreamingResponse<super::node::Fragment>)>> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/FragmentSubscription"),
            streaming: ::grpc::rt::GrpcStreaming::Bidi,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_bidi(o, descriptor)
    }

    pub fn gossip_subscription(&self, o: ::grpc::RequestOptions) -> impl ::std::future::Future<Output=::grpc::Result<(::grpc::ClientRequestSink<super::node::Gossip>, ::grpc::StreamingResponse<super::node::Gossip>)>> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/GossipSubscription"),
            streaming: ::grpc::rt::GrpcStreaming::Bidi,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_bidi(o, descriptor)
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
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/Handshake"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).handshake(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/Tip"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).tip(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/Peers"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).peers(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/GetBlocks"),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |ctx, req, resp| (*handler_copy).get_blocks(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/GetHeaders"),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |ctx, req, resp| (*handler_copy).get_headers(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/GetFragments"),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |ctx, req, resp| (*handler_copy).get_fragments(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/PullHeaders"),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |ctx, req, resp| (*handler_copy).pull_headers(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/PullBlocksToTip"),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |ctx, req, resp| (*handler_copy).pull_blocks_to_tip(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/PushHeaders"),
                        streaming: ::grpc::rt::GrpcStreaming::ClientStreaming,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerClientStreaming::new(move |ctx, req, resp| (*handler_copy).push_headers(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/UploadBlocks"),
                        streaming: ::grpc::rt::GrpcStreaming::ClientStreaming,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerClientStreaming::new(move |ctx, req, resp| (*handler_copy).upload_blocks(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/BlockSubscription"),
                        streaming: ::grpc::rt::GrpcStreaming::Bidi,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerBidi::new(move |ctx, req, resp| (*handler_copy).block_subscription(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/FragmentSubscription"),
                        streaming: ::grpc::rt::GrpcStreaming::Bidi,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerBidi::new(move |ctx, req, resp| (*handler_copy).fragment_subscription(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/iohk.chain.node.Node/GossipSubscription"),
                        streaming: ::grpc::rt::GrpcStreaming::Bidi,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerBidi::new(move |ctx, req, resp| (*handler_copy).gossip_subscription(ctx, req, resp))
                    },
                ),
            ],
        )
    }
}

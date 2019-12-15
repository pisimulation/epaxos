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

pub trait EpaxosService {
    fn write(&self, o: ::grpc::RequestOptions, p: super::epaxos::WriteRequest) -> ::grpc::SingleResponse<super::epaxos::WriteResponse>;

    fn read(&self, o: ::grpc::RequestOptions, p: super::epaxos::ReadRequest) -> ::grpc::SingleResponse<super::epaxos::ReadResponse>;
}

// client

pub struct EpaxosServiceClient {
    grpc_client: ::std::sync::Arc<::grpc::Client>,
    method_write: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::epaxos::WriteRequest, super::epaxos::WriteResponse>>,
    method_read: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::epaxos::ReadRequest, super::epaxos::ReadResponse>>,
}

impl ::grpc::ClientStub for EpaxosServiceClient {
    fn with_client(grpc_client: ::std::sync::Arc<::grpc::Client>) -> Self {
        EpaxosServiceClient {
            grpc_client: grpc_client,
            method_write: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/epaxos.EpaxosService/write".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Unary,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_read: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/epaxos.EpaxosService/read".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Unary,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
        }
    }
}

impl EpaxosService for EpaxosServiceClient {
    fn write(&self, o: ::grpc::RequestOptions, p: super::epaxos::WriteRequest) -> ::grpc::SingleResponse<super::epaxos::WriteResponse> {
        self.grpc_client.call_unary(o, p, self.method_write.clone())
    }

    fn read(&self, o: ::grpc::RequestOptions, p: super::epaxos::ReadRequest) -> ::grpc::SingleResponse<super::epaxos::ReadResponse> {
        self.grpc_client.call_unary(o, p, self.method_read.clone())
    }
}

// server

pub struct EpaxosServiceServer;


impl EpaxosServiceServer {
    pub fn new_service_def<H : EpaxosService + 'static + Sync + Send + 'static>(handler: H) -> ::grpc::rt::ServerServiceDefinition {
        let handler_arc = ::std::sync::Arc::new(handler);
        ::grpc::rt::ServerServiceDefinition::new("/epaxos.EpaxosService",
            vec![
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/epaxos.EpaxosService/write".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |o, p| handler_copy.write(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/epaxos.EpaxosService/read".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |o, p| handler_copy.read(o, p))
                    },
                ),
            ],
        )
    }
}

// interface

pub trait EpaxosInternal {
    fn pre_accept(&self, o: ::grpc::RequestOptions, p: super::epaxos::PreAccept) -> ::grpc::SingleResponse<super::epaxos::PreAcceptOK>;

    fn commit(&self, o: ::grpc::RequestOptions, p: super::epaxos::Commit) -> ::grpc::SingleResponse<super::epaxos::Empty>;
}

// client

pub struct EpaxosInternalClient {
    grpc_client: ::std::sync::Arc<::grpc::Client>,
    method_pre_accept: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::epaxos::PreAccept, super::epaxos::PreAcceptOK>>,
    method_commit: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::epaxos::Commit, super::epaxos::Empty>>,
}

impl ::grpc::ClientStub for EpaxosInternalClient {
    fn with_client(grpc_client: ::std::sync::Arc<::grpc::Client>) -> Self {
        EpaxosInternalClient {
            grpc_client: grpc_client,
            method_pre_accept: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/epaxos.EpaxosInternal/pre_accept".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Unary,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_commit: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/epaxos.EpaxosInternal/commit".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Unary,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
        }
    }
}

impl EpaxosInternal for EpaxosInternalClient {
    fn pre_accept(&self, o: ::grpc::RequestOptions, p: super::epaxos::PreAccept) -> ::grpc::SingleResponse<super::epaxos::PreAcceptOK> {
        self.grpc_client.call_unary(o, p, self.method_pre_accept.clone())
    }

    fn commit(&self, o: ::grpc::RequestOptions, p: super::epaxos::Commit) -> ::grpc::SingleResponse<super::epaxos::Empty> {
        self.grpc_client.call_unary(o, p, self.method_commit.clone())
    }
}

// server

pub struct EpaxosInternalServer;


impl EpaxosInternalServer {
    pub fn new_service_def<H : EpaxosInternal + 'static + Sync + Send + 'static>(handler: H) -> ::grpc::rt::ServerServiceDefinition {
        let handler_arc = ::std::sync::Arc::new(handler);
        ::grpc::rt::ServerServiceDefinition::new("/epaxos.EpaxosInternal",
            vec![
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/epaxos.EpaxosInternal/pre_accept".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |o, p| handler_copy.pre_accept(o, p))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/epaxos.EpaxosInternal/commit".to_string(),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                        resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |o, p| handler_copy.commit(o, p))
                    },
                ),
            ],
        )
    }
}

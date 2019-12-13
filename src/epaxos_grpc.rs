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

pub trait Epaxos {
    fn write(&self, o: ::grpc::RequestOptions, p: super::epaxos::WriteRequest) -> ::grpc::SingleResponse<super::epaxos::WriteResponse>;

    fn read(&self, o: ::grpc::RequestOptions, p: super::epaxos::ReadRequest) -> ::grpc::SingleResponse<super::epaxos::ReadResponse>;
}

// client

pub struct EpaxosClient {
    grpc_client: ::std::sync::Arc<::grpc::Client>,
    method_write: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::epaxos::WriteRequest, super::epaxos::WriteResponse>>,
    method_read: ::std::sync::Arc<::grpc::rt::MethodDescriptor<super::epaxos::ReadRequest, super::epaxos::ReadResponse>>,
}

impl ::grpc::ClientStub for EpaxosClient {
    fn with_client(grpc_client: ::std::sync::Arc<::grpc::Client>) -> Self {
        EpaxosClient {
            grpc_client: grpc_client,
            method_write: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/epaxos.Epaxos/write".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Unary,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
            method_read: ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                name: "/epaxos.Epaxos/read".to_string(),
                streaming: ::grpc::rt::GrpcStreaming::Unary,
                req_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
                resp_marshaller: Box::new(::grpc::protobuf::MarshallerProtobuf),
            }),
        }
    }
}

impl Epaxos for EpaxosClient {
    fn write(&self, o: ::grpc::RequestOptions, p: super::epaxos::WriteRequest) -> ::grpc::SingleResponse<super::epaxos::WriteResponse> {
        self.grpc_client.call_unary(o, p, self.method_write.clone())
    }

    fn read(&self, o: ::grpc::RequestOptions, p: super::epaxos::ReadRequest) -> ::grpc::SingleResponse<super::epaxos::ReadResponse> {
        self.grpc_client.call_unary(o, p, self.method_read.clone())
    }
}

// server

pub struct EpaxosServer;


impl EpaxosServer {
    pub fn new_service_def<H : Epaxos + 'static + Sync + Send + 'static>(handler: H) -> ::grpc::rt::ServerServiceDefinition {
        let handler_arc = ::std::sync::Arc::new(handler);
        ::grpc::rt::ServerServiceDefinition::new("/epaxos.Epaxos",
            vec![
                ::grpc::rt::ServerMethod::new(
                    ::std::sync::Arc::new(::grpc::rt::MethodDescriptor {
                        name: "/epaxos.Epaxos/write".to_string(),
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
                        name: "/epaxos.Epaxos/read".to_string(),
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

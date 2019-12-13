//extern crate clap;
extern crate epaxos_rs;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate protobuf;

//use clap::{App, Arg};
use epaxos_rs::epaxos::*;
use epaxos_rs::epaxos_grpc::*;
use std::{collections::HashMap, thread};

#[derive(Clone)]
struct EpaxosService {
    store: HashMap<String, i32>,
}

impl EpaxosService {
    fn new_store() -> EpaxosService {
        EpaxosService {
            store: HashMap::new(),
        }
    }
}

impl Epaxos for EpaxosService {
    fn write(
        &self,
        _m: grpc::RequestOptions,
        req: WriteRequest,
    ) -> grpc::SingleResponse<WriteResponse> {
        let mut r = WriteResponse::new();

        println!(
            "Received a write request with key = {} and value = {}",
            req.get_key(),
            req.get_value()
        );
        self.store.insert(req.get_key().to_owned(), req.get_value());

        r.set_ack(true);
        grpc::SingleResponse::completed(r)
    }
    fn read(
        &self,
        _m: grpc::RequestOptions,
        req: ReadRequest,
    ) -> grpc::SingleResponse<ReadResponse> {
        let mut r = ReadResponse::new();
        let value = self.store.get(req.get_key());
        r.set_value(*value.unwrap());
        grpc::SingleResponse::completed(r)
    }
}

fn main() {}

//extern crate clap;
extern crate epaxos_rs;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate protobuf;

//use clap::{App, Arg};
use epaxos_rs::epaxos::*;
use epaxos_rs::epaxos_grpc::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct EpaxosService {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    store: Arc<Mutex<HashMap<String, i32>>>,
}

impl EpaxosService {
    fn new_store() -> EpaxosService {
        EpaxosService {
            store: Arc::new(Mutex::new(HashMap::new())),
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
        (*self.store.lock().unwrap()).insert(req.get_key().to_owned(), req.get_value());

        r.set_ack(true);
        grpc::SingleResponse::completed(r)
    }
    fn read(
        &self,
        _m: grpc::RequestOptions,
        req: ReadRequest,
    ) -> grpc::SingleResponse<ReadResponse> {
        let mut r = ReadResponse::new();
        r.set_value(*((*self.store.lock().unwrap()).get(req.get_key())).unwrap());
        grpc::SingleResponse::completed(r)
    }
}

fn main() {
    let mut server_builder = grpc::ServerBuilder::new_plain();
    server_builder.add_service(EpaxosServer::new_service_def(EpaxosService::new_store()));
    server_builder.http.set_port(8080);
    let server = server_builder.build().expect("build");
    println!("server stared on addr {}", server.local_addr());
}

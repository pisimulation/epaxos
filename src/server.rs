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
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    thread,
};

#[derive(Clone)]
struct EpaxosServiceInit {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    store: Arc<Mutex<HashMap<String, i32>>>,
    // cmds: Arc<Mutex<Vec<Vec<Command>>>>, // vectors are growable arrays
    instance_number: Arc<Mutex<i32>>,
}

impl EpaxosServiceInit {
    fn init() -> EpaxosServiceInit {
        EpaxosServiceInit {
            store: Arc::new(Mutex::new(HashMap::new())),
            //    cmds: Arc::new(Mutex::new(Vec::new())),
            instance_number: Arc::new(Mutex::new(0)),
        }
    }

    fn replica_handler() {}

    fn consensus() {}
}

impl EpaxosService for EpaxosServiceInit {
    fn write(
        &self,
        _m: grpc::RequestOptions,
        req: WriteRequest,
    ) -> grpc::SingleResponse<WriteResponse> {
        // TODO: do consensus before committing

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
        // TODO: do consensus before committing

        let mut r = ReadResponse::new();
        r.set_value(*((*self.store.lock().unwrap()).get(req.get_key())).unwrap());
        grpc::SingleResponse::completed(r)
    }
}

fn main() {
    let mut server_builder = grpc::ServerBuilder::new_plain();
    server_builder.add_service(EpaxosServiceServer::new_service_def(
        EpaxosServiceInit::init(),
    ));
    server_builder.http.set_port(8080);
    let server = server_builder.build().expect("build");
    println!("server stared on addr {}", server.local_addr());
    loop {
        thread::park();
    }

    // TODO: handle replica msg here
}

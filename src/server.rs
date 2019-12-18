//extern crate clap;
extern crate epaxos_rs;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate protobuf;

//use clap::{App, Arg};
use epaxos_rs::epaxos::*;
use epaxos_rs::epaxos_grpc::*;
use grpc::ClientStub;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    thread,
};

pub const REPLICA1_PORT: u16 = 10000;
pub const REPLICA2_PORT: u16 = 10001;
pub const REPLICA3_PORT: u16 = 10002;

#[derive(Clone)]
struct Epaxos {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    store: Arc<Mutex<HashMap<String, i32>>>,
    // cmds: Arc<Mutex<Vec<Vec<Command>>>>, // vectors are growable arrays
    instance_number: Arc<Mutex<i32>>,
    replicas: Arc<Mutex<Vec<EpaxosServiceClient>>>,
}

impl Epaxos {
    fn init() -> Epaxos {
        let mut replicas = Vec::new();
        let grpc_replica1 = Arc::new(
            grpc::Client::new_plain("127.0.0.1", REPLICA1_PORT, Default::default()).unwrap(),
        );
        let replica1 = EpaxosServiceClient::with_client(grpc_replica1);
        let grpc_replica2 = Arc::new(
            grpc::Client::new_plain("127.0.0.1", REPLICA2_PORT, Default::default()).unwrap(),
        );
        let replica2 = EpaxosServiceClient::with_client(grpc_replica2);
        let grpc_replica3 = Arc::new(
            grpc::Client::new_plain("127.0.0.1", REPLICA3_PORT, Default::default()).unwrap(),
        );
        let replica3 = EpaxosServiceClient::with_client(grpc_replica3);
        replicas.push(replica1);
        replicas.push(replica2);
        replicas.push(replica3);
        return Epaxos {
            store: Arc::new(Mutex::new(HashMap::new())),
            //    cmds: Arc::new(Mutex::new(Vec::new())),
            instance_number: Arc::new(Mutex::new(0)),
            replicas: Arc::new(Mutex::new(replicas)),
        };
    }

    fn replica_handler() {
        println!("PIPI replica hander");
    }

    fn consensus(&self, write_req: &WriteRequest) {
        for i in 0..2 {
            let mut pre_accept_msg = PreAccept::new();
            pre_accept_msg.set_instance_number(*self.instance_number.lock().unwrap());
            pre_accept_msg.set_write_req(write_req.clone());
            (*self.replicas.lock().unwrap())[i]
                .pre_accept(grpc::RequestOptions::new(), pre_accept_msg);
            println!("replicaaaaa");
        }
        // for replica in *self.replicas.lock().unwrap() {
        //     println!("replica");
        // }
        println!("PIPI consensus");
    }
}

impl EpaxosService for Epaxos {
    fn write(
        &self,
        _m: grpc::RequestOptions,
        req: WriteRequest,
    ) -> grpc::SingleResponse<WriteResponse> {
        // TODO: do consensus before committing
        //(*self.replicas.lock().unwrap())[0].pre_accept();
        self.consensus(&req);

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
    fn pre_accept(
        &self,
        o: grpc::RequestOptions,
        pre_accept_msg: PreAccept,
    ) -> grpc::SingleResponse<PreAcceptOK> {
        println!("PIPI preaccept");
        let mut r = PreAcceptOK::new();
        let mut key = pre_accept_msg.get_write_req().get_key();
        println!("PIPIPIPIPI KEY: {}", key);
        return grpc::SingleResponse::completed(r);
    }
    fn commit(&self, o: grpc::RequestOptions, commit_msg: Commit) -> grpc::SingleResponse<Empty> {
        let mut r = Empty::new();
        return grpc::SingleResponse::completed(r);
    }
}

fn main() {
    let mut server_builder1 = grpc::ServerBuilder::new_plain();
    server_builder1.add_service(EpaxosServiceServer::new_service_def(Epaxos::init()));
    server_builder1.http.set_port(REPLICA1_PORT);
    let server1 = server_builder1.build().expect("build");
    println!("server 1 started on addr {}", server1.local_addr());

    let mut server_builder2 = grpc::ServerBuilder::new_plain();
    server_builder2.add_service(EpaxosServiceServer::new_service_def(Epaxos::init()));
    server_builder2.http.set_port(REPLICA2_PORT);
    let server2 = server_builder2.build().expect("build");
    println!("server 2 started on addr {}", server2.local_addr());

    // let mut server_builder3 = grpc::ServerBuilder::new_plain();
    // server_builder3.add_service(EpaxosServiceServer::new_service_def(Epaxos::init()));
    // server_builder3.http.set_port(REPLICA3_PORT);
    // let server3 = server_builder3.build().expect("build");
    // println!("server 3 started on addr {}", server3.local_addr());
    loop {
        thread::park();
    }

    // TODO: handle replica msg here
}

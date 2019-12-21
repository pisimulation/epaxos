extern crate epaxos_rs;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate protobuf;

use epaxos_rs::epaxos::*;
use epaxos_rs::epaxos_grpc::*;
use grpc::ClientStub;
use std::{
    cmp,
    collections::{HashMap, HashSet},
    env,
    sync::{Arc, Mutex},
    thread,
};

pub const REPLICAS_NUM: u16 = 3;
pub const REPLICA1_PORT: u16 = 10000;
pub const REPLICA2_PORT: u16 = 10001;
pub const REPLICA3_PORT: u16 = 10002;

#[derive(Clone)]
struct Epaxos {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    id: i32,
    store: Arc<Mutex<HashMap<String, i32>>>,
    cmds: Arc<Mutex<Vec<Vec<Command>>>>, // vectors are growable arrays
    instance_number: Arc<Mutex<i32>>,
    replicas: Arc<Mutex<Vec<EpaxosServiceClient>>>,
}

impl Epaxos {
    fn init(id: &i32) -> Epaxos {
        let mut replicas = Vec::new();
        let mut cmds = Vec::new();
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
        cmds.push(Vec::new());
        replicas.push(replica2);
        cmds.push(Vec::new());
        replicas.push(replica3);
        cmds.push(Vec::new());
        return Epaxos {
            id: *id,
            store: Arc::new(Mutex::new(HashMap::new())),
            cmds: Arc::new(Mutex::new(cmds)),
            instance_number: Arc::new(Mutex::new(0)),
            replicas: Arc::new(Mutex::new(replicas)),
        };
    }

    // we only need to do consensus for write req
    fn consensus(&self, write_req: &WriteRequest) {
        println!("Starting consensus");
        let mut pre_accept_msg = PreAccept::new();
        pre_accept_msg.set_replica_id(self.id);
        pre_accept_msg.set_instance_number(*self.instance_number.lock().unwrap());
        pre_accept_msg.set_write_req(write_req.clone());
        let interf = self.find_interference(write_req.get_key().to_owned());
        pre_accept_msg.set_deps(interf.clone());
        let seq = 1 + self.find_max_seq(&interf);
        pre_accept_msg.set_seq(seq);
        let mut cmd = Command::new();
        cmd.set_write_req(write_req.clone());
        cmd.set_seq(seq.clone());
        cmd.set_deps(interf.clone());
        cmd.set_pre_accept(true);
        (*self.cmds.lock().unwrap())[self.id as usize]
            .insert(*self.instance_number.lock().unwrap() as usize, cmd);
        let mut replies = Vec::new();
        for i in 0..REPLICAS_NUM {
            println!("Sending pre_accept to replica {}", i);
            let pre_accept_ok = (*self.replicas.lock().unwrap())[i as usize]
                .pre_accept(grpc::RequestOptions::new(), pre_accept_msg.clone());
            replies.push(pre_accept_ok);
        }
        // TODO how to wait for replies w/o blocking
        //  let res0 = replies[0].wait();
        //println!("Got a pre_accept_ok {:?}", res0);
        *self.instance_number.lock().unwrap() += 1;
    }

    fn find_max_seq(&self, interf: &protobuf::RepeatedField<Command>) -> i32 {
        let mut seq = 0;
        for cmd in interf {
            if cmd.get_seq() > seq {
                seq = cmd.get_seq();
            }
        }
        return seq;
    }

    fn find_interference(&self, key: String) -> protobuf::RepeatedField<Command> {
        println!("Finding interf");
        let mut interf = protobuf::RepeatedField::new();
        // go through self.cmds and look for those with req of the same key
        // if (*self.cmds.lock().unwrap())[self.id as usize].is_empty() {
        //     println!("nothing in cmds yet");
        //     return interf;
        // }
        for cmd in (*self.cmds.lock().unwrap()[self.id as usize]).iter() {
            if cmd.has_write_req() {
                let req = cmd.get_write_req();
                if req.key == key {
                    interf.push(cmd.clone());
                }
            } else {
                let req = cmd.get_read_req();
                if req.key == key {
                    interf.push(cmd.clone());
                }
            }
        }
        return interf;
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
        println!(
            "Replica {} received a PreAccept from {}\n
            Write Key: {}, value: {}",
            self.id,
            pre_accept_msg.get_replica_id(),
            pre_accept_msg.get_write_req().get_key(),
            pre_accept_msg.get_write_req().get_value()
        );
        let key = pre_accept_msg.get_write_req().get_key();
        let sending_replica_id = pre_accept_msg.get_replica_id();
        let i = pre_accept_msg.get_instance_number();
        let interf = self.find_interference(key.to_owned());
        let seq = cmp::max(pre_accept_msg.get_seq(), 1 + self.find_max_seq(&interf));
        // Union interf with deps
        let mut deps = protobuf::RepeatedField::from_vec(pre_accept_msg.get_deps().to_vec());
        // TODO size of src and dst does not match
        println!("PIiiiiiii");
        for interf_command in interf.iter() {
            if !deps.contains(interf_command) {
                deps.push(interf_command.clone());
            }
        }
        // Add to cmd log
        let mut cmd = Command::new();
        cmd.set_write_req(pre_accept_msg.get_write_req().clone());
        cmd.set_seq(seq);
        cmd.set_deps(deps.clone());
        cmd.set_pre_accept(true);
        (*self.cmds.lock().unwrap())[sending_replica_id as usize].insert(i as usize, cmd);

        let mut r = PreAcceptOK::new();
        r.set_replica_id(self.id);
        r.set_write_req(pre_accept_msg.get_write_req().clone());
        r.set_seq(seq);
        r.set_deps(deps.clone());
        r.set_instance_number(i);
        return grpc::SingleResponse::completed(r);
    }
    fn commit(&self, o: grpc::RequestOptions, commit_msg: Commit) -> grpc::SingleResponse<Empty> {
        let mut r = Empty::new();
        return grpc::SingleResponse::completed(r);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let id = &args[1].parse().unwrap();
    let port = &args[2].parse().unwrap();
    let mut server_builder1 = grpc::ServerBuilder::new_plain();
    server_builder1.add_service(EpaxosServiceServer::new_service_def(Epaxos::init(&id)));
    server_builder1.http.set_port(*port);
    let server1 = server_builder1.build().expect("build");
    println!("server 1 started on addr {}", server1.local_addr());

    // Blocks the main thread forever
    loop {
        thread::park();
    }
}

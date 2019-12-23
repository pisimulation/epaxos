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

pub const QUORUM: u16 = 2;
pub const REPLICAS_NUM: u16 = 3;
pub const REPLICA1_PORT: u16 = 10000;
pub const REPLICA2_PORT: u16 = 10001;
pub const REPLICA3_PORT: u16 = 10002;
pub const REPLICA4_PORT: u16 = 10003;
pub const REPLICA5_PORT: u16 = 10004;

#[derive(Debug)]
enum State {
    PreAccept,
    Commit,
}

#[derive(Clone)]
struct Epaxos {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    id: i32,
    store: Arc<Mutex<HashMap<String, i32>>>,
    cmds: Arc<Mutex<Vec<Vec<(Payload, State)>>>>, // vectors are growable arrays
    instance_number: Arc<Mutex<i32>>,
    replicas: Arc<Mutex<Vec<EpaxosInternalClient>>>,
}

impl Epaxos {
    fn init(id: &i32) -> Epaxos {
        let mut replicas = Vec::new();
        let mut cmds = Vec::new();
        let grpc_replica1 = Arc::new(
            grpc::Client::new_plain("127.0.0.1", REPLICA1_PORT, Default::default()).unwrap(),
        );
        let replica1 = EpaxosInternalClient::with_client(grpc_replica1);
        let grpc_replica2 = Arc::new(
            grpc::Client::new_plain("127.0.0.1", REPLICA2_PORT, Default::default()).unwrap(),
        );
        let replica2 = EpaxosInternalClient::with_client(grpc_replica2);
        let grpc_replica3 = Arc::new(
            grpc::Client::new_plain("127.0.0.1", REPLICA3_PORT, Default::default()).unwrap(),
        );
        let replica3 = EpaxosInternalClient::with_client(grpc_replica3);
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
        let slot = *self.instance_number.lock().unwrap();
        let mut payload = Payload::new();
        let mut instance = Instance::new();
        instance.set_replica(self.id);
        instance.set_slot(*self.instance_number.lock().unwrap());
        payload.set_instance(instance);
        payload.set_write_req(write_req.clone());
        let interf = self.find_interference(write_req.get_key().to_owned());
        payload.set_deps(interf.clone());
        let seq = 1 + self.find_max_seq(&interf);
        payload.set_seq(seq);
        (*self.cmds.lock().unwrap())[self.id as usize]
            .insert(slot as usize, (payload.clone(), State::PreAccept));
        let mut fast_quorum = 1; // Leader votes for itself
                                 // TODO: only send to set of fast quorum
        for i in 0..REPLICAS_NUM {
            if i == self.id as u16 {
                continue;
            }
            println!("Sending pre_accept to replica {}", i);
            let pre_accept_ok = (*self.replicas.lock().unwrap())[i as usize]
                .pre_accept(grpc::RequestOptions::new(), payload.clone());
            match pre_accept_ok.wait() {
                Err(e) => panic!("Replica panic {:?}", e),
                Ok((_, value, _))
                    if value.get_seq() == payload.get_seq()
                        && value.get_deps() == payload.get_deps() =>
                {
                    println!("Got an agreeing PreAcceptOK: {:?}", value);
                    fast_quorum += 1;
                }
                // TODO: slow path here
                Ok((_, value, _)) => println!("Some dissenting voice here! {:?}", value),
            }
        }

        // Commit stage if has quorum
        if fast_quorum >= QUORUM {
            // Update the state in the log to commit
            (*self.cmds.lock().unwrap())[self.id as usize][slot as usize].1 = State::Commit;

            // Send Commit message to all replicas
            for i in 0..REPLICAS_NUM {
                if i == self.id as u16 {
                    continue;
                }
                (*self.replicas.lock().unwrap())[i as usize]
                    .commit(grpc::RequestOptions::new(), payload.clone());
                println!("Sending Commit to replica {}", i);
            }
        }
        // TODO: how to wait for replies w/o blocking
        *self.instance_number.lock().unwrap() += 1;
    }

    fn find_max_seq(&self, interf: &protobuf::RepeatedField<Instance>) -> i32 {
        let mut seq = 0;
        for instance in interf {
            let interf_seq = (*self.cmds.lock().unwrap())[instance.get_replica() as usize]
                [instance.get_slot() as usize]
                .0
                .get_seq();
            if interf_seq > seq {
                seq = interf_seq;
            }
        }
        return seq;
    }

    fn find_interference(&self, key: String) -> protobuf::RepeatedField<Instance> {
        println!("Finding interf");
        let mut interf = protobuf::RepeatedField::new();
        for (cmd, state) in (*self.cmds.lock().unwrap()[self.id as usize]).iter() {
            let req = cmd.get_write_req();
            if req.key == key {
                interf.push(cmd.get_instance().clone());
            }
        }
        return interf;
    }

    fn execute(&self) {
        println!("Executing");
    }
}

impl EpaxosExternal for Epaxos {
    fn write(
        &self,
        _m: grpc::RequestOptions,
        req: WriteRequest,
    ) -> grpc::SingleResponse<WriteResponse> {
        println!(
            "Received a write request with key = {} and value = {}",
            req.get_key(),
            req.get_value()
        );
        self.consensus(&req);
        // TODO when do I actually execute?
        (*self.store.lock().unwrap()).insert(req.get_key().to_owned(), req.get_value());
        println!("Consensus successful. Sending a commit to client.");
        let mut r = WriteResponse::new();
        r.set_commit(true);
        grpc::SingleResponse::completed(r)
    }
    fn read(
        &self,
        _m: grpc::RequestOptions,
        req: ReadRequest,
    ) -> grpc::SingleResponse<ReadResponse> {
        println!("Received a read request with key = {}", req.get_key());
        self.execute();
        let mut r = ReadResponse::new();
        r.set_value(*((*self.store.lock().unwrap()).get(req.get_key())).unwrap());
        grpc::SingleResponse::completed(r)
    }
}

impl EpaxosInternal for Epaxos {
    fn pre_accept(
        &self,
        o: grpc::RequestOptions,
        payload: Payload,
    ) -> grpc::SingleResponse<Payload> {
        println!(
            "Replica {} received a PreAccept from {}\n
            Write Key: {}, value: {}",
            self.id,
            payload.get_instance().get_replica(),
            payload.get_write_req().get_key(),
            payload.get_write_req().get_value()
        );
        let key = payload.get_write_req().get_key();
        let sending_replica_id = payload.get_instance().get_replica();
        let slot = payload.get_instance().get_slot();
        let interf = self.find_interference(key.to_owned());
        let seq = cmp::max(payload.get_seq(), 1 + self.find_max_seq(&interf));
        // Union interf with deps
        let mut deps = protobuf::RepeatedField::from_vec(payload.get_deps().to_vec());
        for interf_command in interf.iter() {
            if !deps.contains(interf_command) {
                deps.push(interf_command.clone());
            }
        }
        // Add to cmd log
        (*self.cmds.lock().unwrap())[sending_replica_id as usize]
            .insert(slot as usize, (payload.clone(), State::PreAccept));

        let mut r = payload.clone();
        r.set_seq(seq);
        r.set_deps(deps.clone());
        return grpc::SingleResponse::completed(r);
    }

    fn accept(
        &self,
        o: grpc::RequestOptions,
        payload: Payload,
    ) -> grpc::SingleResponse<AcceptOKPayload> {
        println!("accept");
        let mut r = AcceptOKPayload::new();
        return grpc::SingleResponse::completed(r);
    }
    fn commit(&self, o: grpc::RequestOptions, payload: Payload) -> grpc::SingleResponse<Empty> {
        println!(
            "Replica {} received a Commit from {}\n
            Write Key: {}, value: {}",
            self.id,
            payload.get_instance().get_replica(),
            payload.get_write_req().get_key(),
            payload.get_write_req().get_value()
        );
        // Update the state in the log to commit
        (*self.cmds.lock().unwrap())[payload.get_instance().get_replica() as usize]
            [payload.get_instance().get_slot() as usize]
            .1 = State::Commit;
        println!("My log is {:?}", *self.cmds.lock().unwrap());

        let mut r = Empty::new();
        return grpc::SingleResponse::completed(r);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let id = &args[1].parse().unwrap();
    let port = &args[2].parse().unwrap();
    let mut server_builder1 = grpc::ServerBuilder::new_plain();
    server_builder1.add_service(EpaxosExternalServer::new_service_def(Epaxos::init(&id)));
    server_builder1.add_service(EpaxosInternalServer::new_service_def(Epaxos::init(&id)));
    server_builder1.http.set_port(*port);
    let server1 = server_builder1.build().expect("build");
    println!("server started on addr {}", server1.local_addr());

    // Blocks the main thread forever
    loop {
        thread::park();
    }
}

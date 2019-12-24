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

const SLOW_QUORUM: usize = 3; // floor(N/2)
const FAST_QUORUM: usize = 3; // 1 + floor(F+1/2)
const REPLICAS_NUM: usize = 5;
const LOCALHOST: &str = "127.0.0.1";
static REPLICA_INTERNAL_PORTS: &'static [u16] = &[10000, 10001, 10002, 10003, 10004];
static REPLICA_EXTERNAL_PORTS: &'static [u16] = &[10000, 10001, 10002, 10003, 10004];

#[derive(PartialEq, Eq, Hash, Clone)]
struct ReplicaId(usize);

#[derive(Debug)]
enum State {
    PreAccepted,
    Accepted,
    Committed,
}

#[derive(Clone)]
struct Epaxos {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    id: ReplicaId,
    store: Arc<Mutex<HashMap<String, i32>>>,
    cmds: Arc<Mutex<Vec<Vec<LogEntry>>>>, // vectors are growable arrays
    instance_number: Arc<Mutex<u32>>,
    replicas: Arc<Mutex<HashMap<ReplicaId, EpaxosInternalClient>>>,
}

#[derive(Debug)]
struct LogEntry {
    key: String,
    value: i32,
    seq: u32,
    deps: protobuf::RepeatedField<Instance>,
    state: State,
}

#[derive(Debug)]
struct CommandInstance {
    replica: usize,
    slot: usize,
}

impl Epaxos {
    fn init(id: ReplicaId) -> Epaxos {
        let mut commands = Vec::new();
        let mut replicas = HashMap::new();

        for i in 0..REPLICAS_NUM {
            commands.push(Vec::new());

            if i != id.0 {
                let internal_client = grpc::Client::new_plain(
                    LOCALHOST,
                    REPLICA_INTERNAL_PORTS[i as usize],
                    Default::default(),
                )
                .unwrap();
                let replica = EpaxosInternalClient::with_client(Arc::new(internal_client));
                replicas.insert(ReplicaId(i), replica);
            }
        }

        return Epaxos {
            id: id.clone(),
            store: Arc::new(Mutex::new(HashMap::new())),
            cmds: Arc::new(Mutex::new(commands)),
            instance_number: Arc::new(Mutex::new(0)),
            replicas: Arc::new(Mutex::new(replicas)),
        };
    }

    // we only need to do consensus for write req
    fn consensus(&self, write_req: &WriteRequest) -> bool {
        println!("Starting consensus");
        let slot = *self.instance_number.lock().unwrap();
        let mut payload = Payload::new();
        let mut instance = Instance::new();
        instance.set_replica(self.id.0 as u32);
        instance.set_slot(*self.instance_number.lock().unwrap());
        payload.set_instance(instance);
        payload.set_write_req(write_req.clone());
        let mut interf = self.find_interference(write_req.get_key().to_owned());
        payload.set_deps(interf.clone());
        let mut seq = 1 + self.find_max_seq(&interf);
        payload.set_seq(seq);
        let log_entry = LogEntry {
            key: write_req.get_key().to_owned(),
            value: write_req.get_value(),
            seq: seq,
            deps: interf.clone(),
            state: State::PreAccepted,
        };
        (*self.cmds.lock().unwrap())[self.id.0 as usize].insert(slot as usize, log_entry);
        let mut fast_quorum = 1; // Leader votes for itself
        let mut slow_path = false;
        let mut fast_path = false;
        // Send PreAccept message to replicas in Fast Quorum
        for i in 0..FAST_QUORUM {
            if i == self.id.0 {
                continue;
            }
            println!("Sending pre_accept to replica {}", i);
            let pre_accept_ok = (*self.replicas.lock().unwrap())
                .get(&ReplicaId(i))
                .unwrap()
                .pre_accept(grpc::RequestOptions::new(), payload.clone());
            match pre_accept_ok.wait() {
                Err(e) => panic!("[PreAccept Stage] Replica panic {:?}", e),
                Ok((_, value, _))
                    if value.get_seq() == payload.get_seq()
                        && value.get_deps() == payload.get_deps() =>
                {
                    println!("Got an agreeing PreAcceptOK: {:?}", value);
                    fast_quorum += 1;
                }
                // TODO: slow path here
                Ok((_, value, _)) => {
                    slow_path = true;
                    println!("Some dissenting voice here! {:?}", value);
                    // Union deps from all replies
                    let mut new_deps = protobuf::RepeatedField::from_vec(value.get_deps().to_vec());
                    for new_dep in new_deps.iter() {
                        if !interf.contains(new_dep) {
                            interf.push(new_dep.clone());
                        }
                    }
                    // Set seq to max of seq from all replies
                    if value.get_seq() > seq {
                        seq = value.get_seq();
                    }
                    payload.set_deps(new_deps);
                    payload.set_seq(seq);
                }
            }
        }

        if fast_quorum >= FAST_QUORUM {
            fast_path = true;
        }

        if slow_path {
            // Update the state of the command in the slot to Accepted
            (*self.cmds.lock().unwrap())[self.id.0 as usize][slot as usize].state = State::Accepted;
            // Run Paxos-Accept phase for new deps and new seq
            // Send Accept message to at least floor(N/2) other replicas
            let mut accept_ok_count = 0;
            for i in 0..REPLICAS_NUM {
                let accept_ok = (*self.replicas.lock().unwrap())
                    .get(&ReplicaId(i))
                    .unwrap()
                    .accept(grpc::RequestOptions::new(), payload.clone());
                match accept_ok.wait() {
                    Err(e) => panic!("[Paxos-Accept Stage] Replica panic {:?}", e),
                    Ok((_, value, _)) => {
                        accept_ok_count += 1;
                    }
                }
            }
            if accept_ok_count >= SLOW_QUORUM {
                fast_path = true;
            }
        }

        // Commit stage if has quorum
        if fast_path {
            // Update the state in the log to commit
            (*self.cmds.lock().unwrap())[self.id.0 as usize][slot as usize].state =
                State::Committed;

            // Send Commit message to all replicas
            for i in 0..REPLICAS_NUM {
                if i == self.id.0 {
                    continue;
                }
                (*self.replicas.lock().unwrap())
                    .get(&ReplicaId(i))
                    .unwrap()
                    .commit(grpc::RequestOptions::new(), payload.clone());
                println!("Sending Commit to replica {}", i);
            }
            *self.instance_number.lock().unwrap() += 1;
            return true;
        }
        println!("Consensus failed.");
        return false;
        // TODO: how to wait for replies w/o blocking
    }

    fn find_max_seq(&self, interf: &protobuf::RepeatedField<Instance>) -> u32 {
        let mut seq = 0;
        for instance in interf {
            let interf_seq = (*self.cmds.lock().unwrap())[instance.get_replica() as usize]
                [instance.get_slot() as usize]
                .seq;
            if interf_seq > seq {
                seq = interf_seq;
            }
        }
        return seq;
    }

    fn find_interference(&self, key: String) -> protobuf::RepeatedField<Instance> {
        println!("Finding interf");
        let mut interf = protobuf::RepeatedField::new();
        for replica in 0..REPLICAS_NUM {
            for (slot, log_entry) in (*self.cmds.lock().unwrap()[replica]).iter().enumerate() {
                if log_entry.key == key {
                    let mut instance = Instance::new();
                    instance.set_replica(replica as u32);
                    instance.set_slot(slot as u32);
                    interf.push(instance);
                }
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
        let mut r = WriteResponse::new();
        if self.consensus(&req) {
            // TODO when do I actually execute?
            (*self.store.lock().unwrap()).insert(req.get_key().to_owned(), req.get_value());
            println!("Consensus successful. Sending a commit to client.");
            r.set_commit(true);
        } else {
            println!("Consensus failed. Notifying client.");
            r.set_commit(false);
        }
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
            self.id.0,
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
        let log_entry = LogEntry {
            key: key.to_owned(),
            value: payload.get_write_req().get_value(),
            seq: seq,
            deps: interf,
            state: State::PreAccepted,
        };
        (*self.cmds.lock().unwrap())[sending_replica_id as usize].insert(slot as usize, log_entry);

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
        println!(
            "Replica {} received an Accept from {}\n",
            self.id.0,
            payload.get_instance().get_replica()
        );
        let log_entry = LogEntry {
            key: payload.get_write_req().get_key().to_owned(),
            value: payload.get_write_req().get_value(),
            seq: payload.get_seq(),
            deps: protobuf::RepeatedField::from_vec(payload.get_deps().to_vec()),
            state: State::Accepted,
        };
        (*self.cmds.lock().unwrap())[payload.get_instance().get_replica() as usize]
            [payload.get_instance().get_slot() as usize] = log_entry;
        let mut r = AcceptOKPayload::new();
        r.set_command(payload.get_write_req().clone());
        r.set_instance(payload.get_instance().clone());
        return grpc::SingleResponse::completed(r);
    }

    fn commit(&self, o: grpc::RequestOptions, payload: Payload) -> grpc::SingleResponse<Empty> {
        println!(
            "Replica {} received a Commit from {}\n
            Write Key: {}, value: {}",
            self.id.0,
            payload.get_instance().get_replica(),
            payload.get_write_req().get_key(),
            payload.get_write_req().get_value()
        );
        // Update the state in the log to commit
        (*self.cmds.lock().unwrap())[payload.get_instance().get_replica() as usize]
            [payload.get_instance().get_slot() as usize]
            .state = State::Committed;
        println!("My log is {:?}", *self.cmds.lock().unwrap());

        let mut r = Empty::new();
        return grpc::SingleResponse::completed(r);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let id: u32 = args[1].parse().unwrap();
    let internal_port = REPLICA_INTERNAL_PORTS[id as usize];
    let external_port = REPLICA_EXTERNAL_PORTS[id as usize];
    let mut server_builder1 = grpc::ServerBuilder::new_plain();
    server_builder1.add_service(EpaxosExternalServer::new_service_def(Epaxos::init(
        ReplicaId(id as usize),
    )));
    server_builder1.add_service(EpaxosInternalServer::new_service_def(Epaxos::init(
        ReplicaId(id as usize),
    )));
    server_builder1.http.set_port(internal_port);
    let server1 = server_builder1.build().expect("build");
    println!("server started on addr {}", server1.local_addr());

    // Blocks the main thread forever
    loop {
        thread::park();
    }
}

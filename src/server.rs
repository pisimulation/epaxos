extern crate crossbeam;
extern crate futures;
extern crate futures_cpupool;
extern crate grpc;
extern crate protobuf;
extern crate sharedlib;

use crossbeam::thread as crossbeam_thread;
use grpc::ClientStub;
use sharedlib::epaxos as grpc_service;
use sharedlib::epaxos_grpc::{EpaxosService, EpaxosServiceClient, EpaxosServiceServer};
use sharedlib::util::*;
use std::{
    cmp,
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
    thread,
};

// TODO be careful of deadlocks, acquire in order pls
// avoid recursive locking (locking twice in same thread)

#[derive(Clone)]
struct Epaxos {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    id: ReplicaId,
    store: Arc<Mutex<HashMap<String, i32>>>,
    cmds: Arc<Mutex<Vec<HashMap<usize, LogEntry>>>>,
    instance_number: Arc<Mutex<u32>>,
    replicas: Arc<Mutex<HashMap<ReplicaId, EpaxosServiceClient>>>,
}

impl Epaxos {
    fn init(id: ReplicaId) -> Epaxos {
        let mut commands = Vec::new();
        let mut replicas = HashMap::new();
        println!("Initializing Replica {}", id.0);
        for i in 0..REPLICAS_NUM {
            commands.insert(i, HashMap::new());
            if i == id.0 {
                continue;
            }
            let internal_client = grpc::Client::new_plain(
                LOCALHOST,
                REPLICA_INTERNAL_PORTS[i as usize],
                Default::default(),
            )
            .unwrap();
            println!(
                ">> Neighbor replica {} created : {:?}",
                i, REPLICA_INTERNAL_PORTS[i as usize]
            );
            let replica = EpaxosServiceClient::with_client(Arc::new(internal_client));
            replicas.insert(ReplicaId(i), replica);
        }

        Epaxos {
            id: id.clone(),
            store: Arc::new(Mutex::new(HashMap::new())),
            cmds: Arc::new(Mutex::new(commands)),
            instance_number: Arc::new(Mutex::new(0)),
            replicas: Arc::new(Mutex::new(replicas)),
        }
    }

    fn fast_quorum(&self) -> Vec<ReplicaId> {
        let mut quorum = Vec::new();
        for i in 1..FAST_QUORUM {
            let mut quorum_member = (self.id.0 as i32 - i as i32).abs();
            if quorum_member == self.id.0 as i32 {
                println!("!!");
                quorum_member = (self.id.0 as i32 - i as i32 - 1).abs();
            }
            quorum.push(ReplicaId(quorum_member as usize));
        }
        quorum
    }

    // we only need to do consensus for write req
    fn consensus(&self, write_req: &WriteRequest) -> bool {
        println!("Starting consensus");
        let slot = *self.instance_number.lock().unwrap();
        let mut interf = self.find_interference(write_req.key.to_owned());
        let mut seq = 1 + self.find_max_seq(&interf);
        let mut payload = Payload {
            write_req: write_req.clone(),
            seq: seq,
            deps: interf.clone(),
            instance: Instance {
                replica: self.id.0,
                slot: slot as usize,
            },
        };
        let mut log_entry = LogEntry {
            key: write_req.key.to_owned(),
            value: write_req.value,
            seq: seq,
            deps: interf.clone(),
            state: State::PreAccepted,
        };
        (*self.cmds.lock().unwrap())[self.id.0].insert(slot as usize, log_entry.clone());
        let mut good_pre_accept_ok_counts = 1; // Leader votes for itself
        let mut slow_path = false;
        let mut fast_path = false;
        let fast_quorum_members = self.fast_quorum();
        // Send PreAccept message to replicas in Fast Quorum
        for replica_id in fast_quorum_members.iter() {
            println!(
                "Replica {} Sending pre_accept to replica {}",
                self.id.0, replica_id.0
            );
            // TODO : continue here, refactor, this is too big, also becareful
            // take a look at futures & rayon
            crossbeam_thread::scope(|s| {
                s.spawn(|_| {
                    let pre_accept_ok = (*self.replicas.lock().unwrap())
                        .get(replica_id)
                        .unwrap()
                        .pre_accept(grpc::RequestOptions::new(), to_grpc_payload(&payload));
                    match pre_accept_ok.wait() {
                        Err(e) => panic!("[PreAccept Stage] Replica panic {:?}", e),
                        Ok((_, value, _))
                            if value.get_seq() == payload.seq
                                && value.get_deps() == to_grpc_payload(&payload).get_deps() =>
                        {
                            println!("Got an agreeing PreAcceptOK: {:?}", value);
                            good_pre_accept_ok_counts += 1;
                        }
                        // TODO: slow path here
                        Ok((_, value, _)) => {
                            slow_path = true;
                            println!("Some dissenting voice here! {:?}", value);
                            // Union deps from all replies
                            let mut new_deps = value
                                .get_deps()
                                .to_vec()
                                .iter()
                                .map(from_grpc_instance)
                                .collect();
                            interf.append(&mut new_deps);
                            interf.sort_by(sort_instances);
                            interf.dedup();
                            // Set seq to max of seq from all replies
                            if value.get_seq() > seq {
                                seq = value.get_seq();
                            }
                            log_entry.deps = interf.clone();
                            (*self.cmds.lock().unwrap())[self.id.0]
                                .insert(slot as usize, log_entry.clone());
                            payload.deps = interf.clone();
                            payload.seq = seq;
                        }
                    }
                });
            })
            .unwrap();
        }
        if good_pre_accept_ok_counts == FAST_QUORUM {
            fast_path = true;
        }
        if slow_path {
            log_entry.state = State::Accepted;
            // Update the state of the command in the slot to Accepted
            (*self.cmds.lock().unwrap())[self.id.0].insert(slot as usize, log_entry.clone());
            // Run Paxos-Accept phase for new deps and new seq
            // Send Accept message to at least floor(N/2) other replicas
            let mut accept_ok_count = 1;
            for replica_id in fast_quorum_members.iter() {
                println!(
                    "Replica {} sending ACCEPT to replica {}",
                    self.id.0, replica_id.0
                );
                crossbeam_thread::scope(|s| {
                    s.spawn(|_| {
                        let accept_ok = (*self.replicas.lock().unwrap())
                            .get(replica_id)
                            .unwrap()
                            .accept(grpc::RequestOptions::new(), to_grpc_payload(&payload));
                        match accept_ok.wait() {
                            Err(e) => panic!("[Paxos-Accept Stage] Replica panic {:?}", e),
                            Ok((_, value, _)) => {
                                println!("[Paxos-Accept Stage] got {:?}", value);
                                accept_ok_count += 1;
                            }
                        }
                    });
                })
                .unwrap();
            }
            if accept_ok_count >= SLOW_QUORUM {
                fast_path = true;
            }
        }

        // Commit stage if has quorum
        if fast_path {
            // Update the state in the log to commit
            log_entry.state = State::Committed;
            (*self.cmds.lock().unwrap())[self.id.0].insert(slot as usize, log_entry);

            // Send Commit message to all replicas
            for replica_id in fast_quorum_members.iter() {
                crossbeam_thread::scope(|s| {
                    s.spawn(|_| {
                        (*self.replicas.lock().unwrap())
                            .get(replica_id)
                            .unwrap()
                            .commit(grpc::RequestOptions::new(), to_grpc_payload(&payload));
                        println!("Sending Commit to replica {}", replica_id.0);
                    });
                })
                .unwrap();
            }
            *self.instance_number.lock().unwrap() += 1;
            println!("My log is {:?}", *self.cmds.lock().unwrap());
            return true;
        }
        println!("Consensus failed.");
        return false;
    }

    fn find_max_seq(&self, interf: &Vec<Instance>) -> u32 {
        let mut seq = 0;
        for instance in interf {
            let interf_seq = (*self.cmds.lock().unwrap())[instance.replica as usize]
                .get(&(instance.slot as usize))
                .unwrap()
                .seq;
            if interf_seq > seq {
                seq = interf_seq;
            }
        }
        seq
    }

    fn find_interference(&self, key: String) -> Vec<Instance> {
        println!("Finding interf");
        let mut interf = Vec::new();
        for replica in 0..REPLICAS_NUM {
            for (slot, log_entry) in (*self.cmds.lock().unwrap())[replica].iter() {
                if log_entry.key == key {
                    let instance = Instance {
                        replica: replica,
                        slot: *slot,
                    };
                    interf.push(instance);
                }
            }
        }
        println!(">> Found interf : {:?}", interf);
        interf
    }

    fn execute(&self) {
        println!("Executing");
    }
}

impl EpaxosService for Epaxos {
    fn write(
        &self,
        _m: grpc::RequestOptions,
        req: grpc_service::WriteRequest,
    ) -> grpc::SingleResponse<grpc_service::WriteResponse> {
        println!(
            "Received a write request with key = {} and value = {}",
            req.get_key(),
            req.get_value()
        );
        let mut r = grpc_service::WriteResponse::new();
        if self.consensus(&from_grpc_write_request(&req)) {
            // TODO when do I actually execute?
            (*self.store.lock().unwrap()).insert(req.get_key().to_owned(), req.get_value());
            println!("Consensus successful. Sending a commit to client\n\n\n\n.");
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
        req: grpc_service::ReadRequest,
    ) -> grpc::SingleResponse<grpc_service::ReadResponse> {
        println!("Received a read request with key = {}", req.get_key());
        self.execute();
        let mut r = grpc_service::ReadResponse::new();
        r.set_value(*((*self.store.lock().unwrap()).get(req.get_key())).unwrap());
        grpc::SingleResponse::completed(r)
    }

    fn pre_accept(
        &self,
        _o: grpc::RequestOptions,
        payload: grpc_service::Payload,
    ) -> grpc::SingleResponse<grpc_service::Payload> {
        println!("=====PRE==ACCEPT========");
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
        let mut deps: Vec<Instance> = payload
            .get_deps()
            .to_vec()
            .iter()
            .map(from_grpc_instance)
            .collect();
        deps.append(&mut interf.clone());
        deps.sort_by(sort_instances);
        deps.dedup();
        // Add to cmd log
        let log_entry = LogEntry {
            key: key.to_owned(),
            value: payload.get_write_req().get_value(),
            seq: seq,
            deps: deps.clone(),
            state: State::PreAccepted,
        };
        (*self.cmds.lock().unwrap())[sending_replica_id as usize].insert(slot as usize, log_entry);

        let mut r = payload.clone();
        r.set_seq(seq);
        r.set_deps(protobuf::RepeatedField::from_vec(
            deps.clone().iter().map(to_grpc_instance).collect(),
        ));
        println!("===============");
        grpc::SingleResponse::completed(r)
    }

    fn accept(
        &self,
        _o: grpc::RequestOptions,
        payload: grpc_service::Payload,
    ) -> grpc::SingleResponse<grpc_service::AcceptOKPayload> {
        println!("=======ACCEPT========");
        println!(
            "Replica {} received an Accept from {}\n",
            self.id.0,
            payload.get_instance().get_replica()
        );
        let sending_replica_id = payload.get_instance().get_replica();
        let slot = payload.get_instance().get_slot();
        let log_entry = LogEntry {
            key: payload.get_write_req().get_key().to_owned(),
            value: payload.get_write_req().get_value(),
            seq: payload.get_seq(),
            deps: payload
                .get_deps()
                .to_vec()
                .iter()
                .map(from_grpc_instance)
                .collect(),
            state: State::Accepted,
        };
        (*self.cmds.lock().unwrap())[sending_replica_id as usize].insert(slot as usize, log_entry);
        let mut r = grpc_service::AcceptOKPayload::new();
        r.set_command(payload.get_write_req().clone());
        r.set_instance(payload.get_instance().clone());
        println!("===============");
        grpc::SingleResponse::completed(r)
    }

    fn commit(
        &self,
        _o: grpc::RequestOptions,
        payload: grpc_service::Payload,
    ) -> grpc::SingleResponse<grpc_service::Empty> {
        println!("======COMMIT=========");
        println!(
            "Replica {} received a Commit from {}\n
            Write Key: {}, value: {}",
            self.id.0,
            payload.get_instance().get_replica(),
            payload.get_write_req().get_key(),
            payload.get_write_req().get_value()
        );
        let sending_replica_id = payload.get_instance().get_replica();
        let slot = payload.get_instance().get_slot();
        let log_entry = LogEntry {
            key: payload.get_write_req().get_key().to_owned(),
            value: payload.get_write_req().get_value(),
            seq: payload.get_seq(),
            deps: payload
                .get_deps()
                .to_vec()
                .iter()
                .map(from_grpc_instance)
                .collect(),
            state: State::Committed,
        };
        // Update the state in the log to commit
        (*self.cmds.lock().unwrap())[sending_replica_id as usize].insert(slot as usize, log_entry);
        println!("My log is {:?}", *self.cmds.lock().unwrap());
        println!("===============");
        let r = grpc_service::Empty::new();
        grpc::SingleResponse::completed(r)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let id: u32 = args[1].parse().unwrap();
    let port = REPLICA_INTERNAL_PORTS[id as usize];
    let mut server_builder1 = grpc::ServerBuilder::new_plain();
    server_builder1.add_service(EpaxosServiceServer::new_service_def(Epaxos::init(
        ReplicaId(id as usize),
    )));
    server_builder1.http.set_port(port);
    let server1 = server_builder1.build().expect("build");
    println!(">> Me {}", server1.local_addr());

    // Blocks the main thread forever
    loop {
        thread::park();
    }
}

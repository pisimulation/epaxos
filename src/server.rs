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
use sharedlib::logic::*;
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
    thread,
};

struct EpaxosServer {
    // In grpc, parameters in service are immutable.
    // See https://github.com/stepancheg/grpc-rust/blob/master/docs/FAQ.md
    store: Arc<Mutex<HashMap<String, i32>>>,
    epaxos_logic: Arc<Mutex<EpaxosLogic>>,
    replicas: HashMap<ReplicaId, EpaxosServiceClient>,
    quorum_members: Vec<ReplicaId>,
}

impl EpaxosServer {
    fn init(id: ReplicaId, quorum_members: Vec<ReplicaId>) -> EpaxosServer {
        let mut replicas = HashMap::new();
        println!("Initializing Replica {}", id.0);
        for i in 0..REPLICAS_NUM {
            if i != id.0 as usize {
                let internal_client = grpc::Client::new_plain(
                    REPLICA_ADDRESSES[i as usize],
                    REPLICA_PORT,
                    Default::default(),
                )
                .unwrap();
                println!(
                    ">> Neighbor replica {} created : {:?}",
                    i, REPLICA_ADDRESSES[i as usize]
                );
                let replica = EpaxosServiceClient::with_client(Arc::new(internal_client));
                replicas.insert(ReplicaId(i as u32), replica);
            }
        }

        EpaxosServer {
            store: Arc::new(Mutex::new(HashMap::new())),
            epaxos_logic: Arc::new(Mutex::new(EpaxosLogic::init(id))),
            replicas: replicas,
            quorum_members: quorum_members,
        }
    }

    // we only need to do consensus for write req
    fn consensus(&self, write_req: &WriteRequest) -> bool {
        println!("Starting consensus");
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let payload = epaxos_logic.lead_consensus(write_req.clone());
        let pre_accept_oks = self.send_pre_accepts(&payload);

        match epaxos_logic.decide_path(pre_accept_oks, &payload) {
            Path::Fast(payload_) => {
                // Send Commit message to F
                self.send_commits(&payload_);
                epaxos_logic.committed(payload_);
                return true;
            }
            Path::Slow(payload_) => {
                // Start Paxos-Accept stage
                // Send Accept message to F
                epaxos_logic.accepted(payload_.clone());
                if self.send_accepts(&payload_) >= SLOW_QUORUM {
                    self.send_commits(&payload_);
                    epaxos_logic.committed(payload_);
                    return true;
                }
                return false;
            }
        }
    }

    fn send_pre_accepts(&self, payload: &Payload) -> Vec<Payload> {
        let mut pre_accept_oks = Vec::new();
        for replica_id in self.quorum_members.iter() {
            //println!("Sending PreAccept to {:?}", replica_id);
            crossbeam_thread::scope(|s| {
                s.spawn(|_| {
                    let pre_accept_ok = self
                        .replicas
                        .get(replica_id)
                        .unwrap()
                        .pre_accept(grpc::RequestOptions::new(), payload.to_grpc());
                    match pre_accept_ok.wait() {
                        Err(e) => panic!("[PreAccept Stage] Replica panic {:?}", e),
                        Ok((_, value, _)) => {
                            pre_accept_oks.push(Payload::from_grpc(&value));
                        }
                    }
                });
            })
            .unwrap();
        }
        pre_accept_oks
    }
    fn send_accepts(&self, payload: &Payload) -> usize {
        let mut accept_ok_count: usize = 1;
        for replica_id in self.quorum_members.iter() {
            crossbeam_thread::scope(|s| {
                s.spawn(|_| {
                    let accept_ok = self
                        .replicas
                        .get(replica_id)
                        .unwrap()
                        .accept(grpc::RequestOptions::new(), payload.to_grpc());
                    match accept_ok.wait() {
                        Err(e) => panic!("[Paxos-Accept Stage] Replica panic {:?}", e),
                        Ok((_, _, _)) => {
                            accept_ok_count += 1;
                        }
                    }
                });
            })
            .unwrap();
        }
        accept_ok_count
    }
    fn send_commits(&self, payload: &Payload) {
        for replica_id in self.quorum_members.iter() {
            crossbeam_thread::scope(|s| {
                s.spawn(|_| {
                    self.replicas
                        .get(replica_id)
                        .unwrap()
                        .commit(grpc::RequestOptions::new(), payload.to_grpc());
                    println!("Sending Commit to replica {}", replica_id.0);
                });
            })
            .unwrap();
        }
    }

    fn execute(&self) {
        //println!("Executing");
    }
}

impl EpaxosService for EpaxosServer {
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
        if self.consensus(&WriteRequest::from_grpc(&req)) {
            // TODO when do I actually execute?
            (*self.store.lock().unwrap()).insert(req.get_key().to_owned(), req.get_value());
            println!("DONE my store: {:#?}", self.store.lock().unwrap());
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
        p: grpc_service::Payload,
    ) -> grpc::SingleResponse<grpc_service::Payload> {
        println!("Received PreAccept");
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let request = PreAccept(Payload::from_grpc(&p));
        let response = epaxos_logic.pre_accept_(request);
        grpc::SingleResponse::completed(response.0.to_grpc())
    }

    fn accept(
        &self,
        _o: grpc::RequestOptions,
        p: grpc_service::Payload,
    ) -> grpc::SingleResponse<grpc_service::AcceptOKPayload> {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let request = Accept(Payload::from_grpc(&p));
        let response = epaxos_logic.accept_(request);
        grpc::SingleResponse::completed(response.0.to_grpc())
    }

    fn commit(
        &self,
        _o: grpc::RequestOptions,
        p: grpc_service::Payload,
    ) -> grpc::SingleResponse<grpc_service::Empty> {
        let mut epaxos_logic = self.epaxos_logic.lock().unwrap();
        let request = Commit(Payload::from_grpc(&p));
        epaxos_logic.commit_(request);
        grpc::SingleResponse::completed(grpc_service::Empty::new())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let id: u32 = args[1].parse().unwrap();
    let r1: u32 = args[2].parse().unwrap();
    let r2: u32 = args[3].parse().unwrap();
    let mut server_builder1 = grpc::ServerBuilder::new_plain();
    server_builder1.add_service(EpaxosServiceServer::new_service_def(EpaxosServer::init(
        ReplicaId(id),
        vec![ReplicaId(r1), ReplicaId(r2)],
    )));
    server_builder1.http.set_port(REPLICA_PORT);
    let server1 = server_builder1.build().expect("build");
    println!(">> Me {}", server1.local_addr());

    // Blocks the main thread forever
    loop {
        thread::park();
    }
}

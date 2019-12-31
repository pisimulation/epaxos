extern crate protobuf;

use std::{
    cmp,
    cmp::Ordering,
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

pub const SLOW_QUORUM: usize = 3; // floor(N/2)
pub const FAST_QUORUM: usize = 3; // 1 + floor(F+1/2)
pub const REPLICAS_NUM: usize = 5;
pub const LOCALHOST: &str = "127.0.0.1";
pub static REPLICA_INTERNAL_PORTS: &'static [u16] = &[10000, 10001, 10002, 10003, 10004, 10005];
pub static REPLICA_EXTERNAL_PORTS: &'static [u16] = &[10000, 10001, 10002, 10003, 10004, 10005];

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct ReplicaId(pub usize);

#[derive(Clone)]
pub struct WriteRequest {
    pub key: String,
    pub value: i32,
}

#[derive(Clone)]
pub struct WriteResponse {
    pub commit: bool,
}

#[derive(Clone)]
pub struct ReadRequest {
    pub key: String,
}

#[derive(Clone)]
pub struct ReadResponse {
    pub value: i32,
}

#[derive(Clone)]
pub enum State {
    PreAccepted,
    Accepted,
    Committed,
}

#[derive(Clone)]
pub struct Payload {
    pub write_req: WriteRequest,
    pub seq: u32,
    pub deps: Vec<Instance>,
    pub instance: Instance,
}

#[derive(Clone)]
pub struct AcceptOKPayload {
    pub write_req: WriteRequest,
    pub instance: Instance,
}

#[derive(Clone)]
pub struct LogEntry {
    pub key: String,
    pub value: i32,
    pub seq: u32,
    pub deps: Vec<Instance>,
    pub state: State,
}

#[derive(Clone, PartialEq)]
pub struct Instance {
    pub replica: usize,
    pub slot: usize,
}

pub struct PreAccept(pub Payload);

pub struct Accept(pub Payload);

pub struct Commit(pub Payload);

pub struct PreAcceptOK(pub Payload);

pub struct AcceptOK(pub AcceptOKPayload);

pub enum Path {
    Slow,
    Fast,
}

pub fn sort_instances(inst1: &Instance, inst2: &Instance) -> Ordering {
    if inst1.replica < inst2.replica {
        Ordering::Less
    } else if inst1.replica > inst2.replica {
        Ordering::Greater
    } else {
        if inst1.slot < inst2.slot {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

pub struct EpaxosLogic {
    pub id: ReplicaId,
    pub cmds: Vec<HashMap<usize, LogEntry>>,
    pub instance_number: u32,
}

impl EpaxosLogic {
    pub fn init(id: ReplicaId) -> EpaxosLogic {
        let commands = vec![HashMap::new(); REPLICAS_NUM];
        EpaxosLogic {
            id: id,
            cmds: commands,
            instance_number: 0,
        }
    }

    pub fn update_log(&mut self, log_entry: LogEntry, instance: Instance) {
        self.cmds[instance.replica].insert(instance.slot, log_entry);
    }

    pub fn lead_consensus(&mut self, write_req: WriteRequest, state: State) -> Payload {
        let WriteRequest { key, value } = write_req.clone();
        let slot = self.instance_number;
        let interf = self.find_interference(key.clone());
        let seq = 1 + self.find_max_seq(&interf);
        let log_entry = LogEntry {
            key: key.to_owned(),
            value: value,
            seq: seq,
            deps: interf.clone(),
            state: state,
        };
        self.update_log(
            log_entry,
            Instance {
                replica: self.id.0,
                slot: slot as usize,
            },
        );
        //self.cmds[self.id.0].insert(slot as usize, log_entry);
        Payload {
            write_req: write_req,
            seq: seq,
            deps: interf,
            instance: Instance {
                replica: self.id.0,
                slot: slot as usize,
            },
        }
    }

    pub fn establish_ordering_constraints(
        &self,
        pre_accept_oks: Vec<Payload>,
        payload: Payload,
    ) -> (Path, Payload) {
        let mut path = Path::Fast;
        let mut new_payload = payload;
        for pre_accept_ok in pre_accept_oks {
            let Payload {
                write_req,
                seq,
                deps,
                instance,
            } = pre_accept_ok.clone();
            if seq == payload.seq && deps == payload.deps {
                continue;
            } else {
                path = Path::Slow;
                // Union deps from all replies
                let mut new_deps = pre_accept_ok.deps;
                new_payload.deps.append(&mut new_deps);
                new_payload.deps.sort_by(sort_instances);
                new_payload.deps.dedup();
                new_payload.deps = new_deps.clone();
                // Set seq to max of seq from all replies
                if pre_accept_ok.seq > seq {
                    new_payload.seq = pre_accept_ok.seq;
                }
                // i dont thi nk i need to do this now
                //      log_entry.deps = interf.clone();
                //      (*self.cmds.lock().unwrap())[self.id.0].insert(slot as usize, log_entry.clone());
                //payload.seq = seq;
            }
        }
        (path, new_payload)
    }

    pub fn fast_quorum(&self) -> Vec<ReplicaId> {
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

    pub fn pre_accept_(&mut self, pre_accept_req: PreAccept) -> PreAcceptOK {
        println!("=====PRE==ACCEPT========");
        let Payload {
            write_req,
            seq,
            mut deps,
            instance,
        } = pre_accept_req.0;
        let WriteRequest { key, value } = write_req.clone();
        let Instance { replica, slot } = instance;
        //let key = write_req.key;
        // let sending_replica_id = payload.get_instance().get_replica();
        // let slot = instance.slot;
        let interf = self.find_interference(key.to_owned());
        let seq = cmp::max(seq, 1 + self.find_max_seq(&interf));
        // Union interf with deps
        // let mut deps: Vec<Instance> = deps
        //     .to_vec()
        //     .iter()
        //     .map(Instance::from_grpc)
        //     .collect();
        deps.append(&mut interf.clone());
        deps.sort_by(sort_instances);
        deps.dedup();
        // Add to cmd log
        let log_entry = LogEntry {
            key: key.to_owned(),
            value: value,
            seq: seq,
            deps: deps.clone(),
            state: State::PreAccepted,
        };
        self.cmds[replica as usize].insert(slot as usize, log_entry);

        // let mut r = payload.clone();
        // r.set_seq(seq);
        // r.set_deps(protobuf::RepeatedField::from_vec(
        //     deps.clone().iter().map(Instance::to_grpc).collect(),
        // ));
        println!("===============");
        PreAcceptOK(Payload {
            write_req,
            seq,
            deps,
            instance,
        })
    }
    pub fn accept_(&mut self, accept_req: Accept) -> AcceptOK {
        println!("=======ACCEPT========");
        let Payload {
            write_req,
            seq,
            deps,
            instance,
        } = accept_req.0;
        let WriteRequest { key, value } = write_req.clone();
        let Instance { replica, slot } = instance;
        // println!(
        //     "Replica {} received an Accept from {}\n",
        //     self.id.0,
        //     payload.get_instance().get_replica()
        // );
        //let sending_replica_id = payload.get_instance().get_replica();
        //let slot = payload.get_instance().get_slot();
        let log_entry = LogEntry {
            key: key.to_owned(),
            value: value,
            seq: seq,
            deps: deps,
            state: State::Accepted,
        };
        self.cmds[replica as usize].insert(slot as usize, log_entry);
        // let mut r = grpc_service::AcceptOKPayload::new();
        // r.set_command(payload.get_write_req().clone());
        // r.set_instance(payload.get_instance().clone());
        // println!("===============");
        // grpc::SingleResponse::completed(r)
        AcceptOK(AcceptOKPayload {
            write_req: write_req,
            instance: instance,
        })
    }
    pub fn commit_(&mut self, commit_req: Commit) -> () {
        let Payload {
            write_req,
            seq,
            deps,
            instance,
        } = commit_req.0;
        let WriteRequest { key, value } = write_req;
        let Instance { replica, slot } = instance;
        println!("======COMMIT=========");
        // println!(
        //     "Replica {} received a Commit from {}\n
        //     Write Key: {}, value: {}",
        //     self.id.0,
        //     payload.get_instance().get_replica(),
        //     payload.get_write_req().get_key(),
        //     payload.get_write_req().get_value()
        // );
        // let sending_replica_id = payload.get_instance().get_replica();
        // let slot = payload.get_instance().get_slot();
        let log_entry = LogEntry {
            key: key.to_owned(),
            value: value,
            seq: seq,
            deps: deps,
            state: State::Committed,
        };
        // Update the state in the log to commit
        self.cmds[replica as usize].insert(slot as usize, log_entry);
        println!("My log is {:?}", self.cmds);
        // println!("===============");
        // let r = grpc_service::Empty::new();
        // grpc::SingleResponse::completed(r)
    }

    fn find_interference(&self, key: String) -> Vec<Instance> {
        println!("Finding interf");
        let mut interf = Vec::new();
        for replica in 0..REPLICAS_NUM {
            for (slot, log_entry) in self.cmds[replica].iter() {
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

    fn find_max_seq(&self, interf: &Vec<Instance>) -> u32 {
        let mut seq = 0;
        for instance in interf {
            let interf_seq = self.cmds[instance.replica as usize]
                .get(&(instance.slot as usize))
                .unwrap()
                .seq;
            if interf_seq > seq {
                seq = interf_seq;
            }
        }
        seq
    }
}

impl fmt::Debug for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Key = {}\nValue = {}\nSeq = {}\nDeps = {:?}\nState = {:?}\n",
            self.key, self.value, self.seq, self.deps, self.state
        )
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Replica ID = {}\nSlot = {}", self.replica, self.slot)
    }
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            State::PreAccepted => write!(f, "PreAccepted"),
            State::Accepted => write!(f, "Accepted"),
            State::Committed => write!(f, "Committed"),
        }
    }
}

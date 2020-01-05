extern crate protobuf;

use std::{cmp, cmp::Ordering, collections::HashMap, fmt};

pub const SLOW_QUORUM: usize = 3; // floor(N/2)
pub const FAST_QUORUM: usize = 4; // 2F
pub const REPLICAS_NUM: usize = 5;
pub const LOCALHOST: &str = "127.0.0.1";
pub static REPLICA_INTERNAL_PORTS: &'static [u16] = &[10000, 10001, 10002, 10003, 10004, 10005];
pub static REPLICA_EXTERNAL_PORTS: &'static [u16] = &[10000, 10001, 10002, 10003, 10004, 10005];

#[derive(PartialEq, Eq, Hash, Clone, Debug, Copy)]
pub struct ReplicaId(pub u32);

#[derive(Debug, Clone)]
pub struct WriteRequest {
    pub key: String,
    pub value: i32,
}

#[derive(Clone, Copy)]
pub struct WriteResponse {
    pub commit: bool,
}

#[derive(Clone)]
pub struct ReadRequest {
    pub key: String,
}

#[derive(Clone, Copy)]
pub struct ReadResponse {
    pub value: i32,
}

#[derive(Clone)]
pub enum State {
    PreAccepted,
    Accepted,
    Committed,
}

#[derive(Debug, Clone)]
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

#[derive(Clone, PartialEq, Copy)]
pub struct Instance {
    pub replica: u32,
    pub slot: u32,
}

pub struct PreAccept(pub Payload);

pub struct Accept(pub Payload);

pub struct Commit(pub Payload);

pub struct PreAcceptOK(pub Payload);

pub struct AcceptOK(pub AcceptOKPayload);

pub enum Path {
    Slow(Payload),
    Fast(Payload),
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

    pub fn update_log(&mut self, log_entry: LogEntry, instance: &Instance) {
        self.cmds[instance.replica as usize].insert(instance.slot as usize, log_entry);
    }

    pub fn lead_consensus(&mut self, write_req: WriteRequest) -> Payload {
        // let WriteRequest { key, value } = write_req;
        let slot = self.instance_number;
        let interf = self.find_interference(&write_req.key);
        let seq = 1 + self.find_max_seq(&interf);
        let log_entry = LogEntry {
            key: write_req.key.to_owned(),
            value: write_req.value,
            seq: seq,
            deps: interf.clone(),
            state: State::PreAccepted,
        };
        self.update_log(
            log_entry,
            &Instance {
                replica: self.id.0,
                slot: slot,
            },
        );
        Payload {
            write_req: write_req,
            seq: seq,
            deps: interf,
            instance: Instance {
                replica: self.id.0,
                slot: slot,
            },
        }
    }

    pub fn decide_path(&self, pre_accept_oks: Vec<Payload>, payload: &Payload) -> Path {
        let mut new_payload = payload.clone();
        let mut path = Path::Fast(payload.clone());
        for pre_accept_ok in pre_accept_oks {
            let Payload {
                write_req: _,
                seq,
                deps,
                instance: _,
            } = pre_accept_ok.clone();
            if seq == payload.seq && deps == payload.deps {
                continue;
            } else {
                println!("Got some dissenting voice: {:#?}", pre_accept_ok.deps);
                // Union deps from all replies
                let new_deps = self.union_deps(new_payload.deps, pre_accept_ok.deps);
                new_payload.deps = new_deps.clone();
                // Set seq to max of seq from all replies
                if pre_accept_ok.seq > seq {
                    new_payload.seq = pre_accept_ok.seq;
                }
                path = Path::Slow(new_payload.clone());
            }
        }
        path
    }

    pub fn committed(&mut self, payload: Payload) {
        let Payload {
            write_req,
            seq,
            deps,
            instance,
        } = payload;
        self.instance_number += 1;
        let log_entry = LogEntry {
            key: write_req.key,
            value: write_req.value,
            seq: seq,
            deps: deps,
            state: State::Committed,
        };
        self.update_log(
            log_entry,
            &Instance {
                replica: instance.replica,
                slot: instance.slot,
            },
        );
        println!("Commited. My log is {:#?}", self.cmds);
    }

    pub fn accepted(&mut self, payload: Payload) {
        let Payload {
            write_req,
            seq,
            deps,
            instance,
        } = payload;
        let log_entry = LogEntry {
            key: write_req.key,
            value: write_req.value,
            seq: seq,
            deps: deps,
            state: State::Accepted,
        };
        self.update_log(
            log_entry,
            &Instance {
                replica: instance.replica,
                slot: instance.slot,
            },
        );
    }

    pub fn union_deps(&self, mut deps1: Vec<Instance>, mut deps2: Vec<Instance>) -> Vec<Instance> {
        deps1.append(&mut deps2);
        deps1.sort_by(sort_instances);
        deps1.dedup();
        deps1
    }

    pub fn fast_quorum(&self) -> Vec<ReplicaId> {
        let mut quorum = Vec::new();
        for i in 1..FAST_QUORUM {
            let mut quorum_member = (self.id.0 as i32 - i as i32).abs();
            if quorum_member == self.id.0 as i32 {
                println!("!!");
                quorum_member = (self.id.0 as i32 - i as i32 - 1).abs();
            }
            quorum.push(ReplicaId(quorum_member as u32));
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
        let interf = self.find_interference(&key);
        let seq_ = cmp::max(seq, 1 + self.find_max_seq(&interf));
        if interf != deps {
            deps = self.union_deps(deps, interf);
        }
        let log_entry = LogEntry {
            key: key.to_owned(),
            value: value,
            seq: seq_,
            deps: deps.clone(),
            state: State::PreAccepted,
        };
        self.update_log(log_entry, &instance);
        PreAcceptOK(Payload {
            write_req: write_req,
            seq: seq_,
            deps: deps,
            instance: instance,
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
        let log_entry = LogEntry {
            key: key,
            value: value,
            seq: seq,
            deps: deps,
            state: State::Accepted,
        };
        self.update_log(log_entry, &instance);
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
        println!("======COMMIT=========");
        let log_entry = LogEntry {
            key: write_req.key,
            value: write_req.value,
            seq: seq,
            deps: deps,
            state: State::Committed,
        };
        // Update the state in the log to commit
        self.update_log(log_entry, &instance);
        println!("Committed. My log is {:#?}", self.cmds);
    }

    fn find_interference(&self, key: &String) -> Vec<Instance> {
        println!("Finding interf");
        let mut interf = Vec::new();
        for replica in 0..REPLICAS_NUM {
            for (slot, log_entry) in self.cmds[replica].iter() {
                if log_entry.key == *key {
                    let instance = Instance {
                        replica: replica as u32,
                        slot: *slot as u32,
                    };
                    interf.push(instance);
                }
            }
        }
        interf.sort_by(sort_instances);
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
            "\nWrite(key = {}, value = {})\nSeq = {}\nDeps = {:#?}\nState = {:?}\n",
            self.key, self.value, self.seq, self.deps, self.state
        )
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "(Replica: {}, Slot: {})", self.replica, self.slot)
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

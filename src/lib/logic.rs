// extern crate sharedlib;
use std::{cmp::Ordering, fmt};
extern crate protobuf;

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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub key: String,
    pub value: i32,
    pub seq: u32,
    pub deps: Vec<Instance>,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub replica: usize,
    pub slot: usize,
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

// impl fmt::Display for LogEntry {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         writeln!(
//             f,
//             "Key = {}\nValue = {}\nSeq = {}\nDeps = {:?}\nState = {}\n",
//             self.key, self.value, self.seq, self.deps, self.state
//         )
//     }
// }

// impl fmt::Display for Instance {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         writeln!(f, "Replica ID = {}\nSlot = {}", self.replica, self.slot)
//     }
// }

// impl fmt::Display for State {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match *self {
//             State::PreAccepted => write!(f, "PreAccepted"),
//             State::Accepted => write!(f, "Accepted"),
//             State::Committed => write!(f, "Committed"),
//         }
//     }
// }

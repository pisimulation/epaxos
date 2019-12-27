// extern crate sharedlib;
use crate::epaxos as grpc;
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

pub fn to_grpc_payload(payload: &Payload) -> grpc::Payload {
    let mut grpc_payload = grpc::Payload::new();
    grpc_payload.set_write_req(to_grpc_write_request(&payload.write_req));
    grpc_payload.set_seq(payload.seq);
    grpc_payload.set_deps(protobuf::RepeatedField::from_vec(
        payload.deps.iter().map(to_grpc_instance).collect(),
    ));
    grpc_payload.set_instance(to_grpc_instance(&payload.instance));
    grpc_payload
}

pub fn to_grpc_write_request(write_req: &WriteRequest) -> grpc::WriteRequest {
    let mut grpc_write_req = grpc::WriteRequest::new();
    grpc_write_req.set_key((*write_req.key).to_string());
    grpc_write_req.set_value(write_req.value);
    grpc_write_req
}

pub fn to_grpc_instance(instance: &Instance) -> grpc::Instance {
    let mut grpc_instance = grpc::Instance::new();
    grpc_instance.set_replica(instance.replica as u32);
    grpc_instance.set_slot(instance.slot as u32);
    grpc_instance
}

pub fn from_grpc_instance(grpc_instance: &grpc::Instance) -> Instance {
    Instance {
        replica: grpc_instance.get_replica() as usize,
        slot: grpc_instance.get_slot() as usize,
    }
}

pub fn from_grpc_write_request(grpc_write_req: &grpc::WriteRequest) -> WriteRequest {
    WriteRequest {
        key: grpc_write_req.get_key().to_owned(),
        value: grpc_write_req.get_value(),
    }
}

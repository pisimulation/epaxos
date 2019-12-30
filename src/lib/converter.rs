use crate::epaxos as grpc;
use crate::logic::*;

impl WriteRequest {
    pub fn from_grpc(req: &grpc::WriteRequest) -> Self {
        WriteRequest {
            key: req.get_key().to_owned(),
            value: req.get_value(),
        }
    }

    pub fn to_grpc(&self) -> grpc::WriteRequest {
        let mut req = grpc::WriteRequest::new();
        req.set_key(self.key.to_owned());
        req.set_value(self.value);
        req
    }
}

impl WriteResponse {
    pub fn from_grpc(res: &grpc::WriteResponse) -> Self {
        WriteResponse { commit: res.commit }
    }

    pub fn to_grpc(&self) -> grpc::WriteResponse {
        let mut resp = grpc::WriteResponse::new();
        resp.commit = self.commit;
        resp
    }
}

impl ReadRequest {
    pub fn from_grpc(req: &grpc::ReadRequest) -> Self {
        ReadRequest {
            key: req.get_key().to_owned(),
        }
    }

    pub fn to_grpc(&self) -> grpc::ReadRequest {
        let mut req = grpc::ReadRequest::new();
        req.set_key(self.key.to_owned());
        req
    }
}

impl ReadResponse {
    pub fn from_grpc(res: &grpc::ReadResponse) -> Self {
        ReadResponse { value: res.value }
    }

    pub fn to_grpc(&self) -> grpc::ReadResponse {
        let mut res = grpc::ReadResponse::new();
        res.set_value(self.value);
        res
    }
}

impl Payload {
    pub fn from_grpc(payload: &grpc::Payload) -> Self {
        Payload {
            write_req: WriteRequest::from_grpc(payload.get_write_req()),
            seq: payload.get_seq(),
            deps: payload.get_deps().iter().map(Instance::from_grpc).collect(),
            instance: Instance::from_grpc(payload.get_instance()),
        }
    }

    pub fn to_grpc(&self) -> grpc::Payload {
        let mut payload = grpc::Payload::new();
        payload.set_write_req(self.write_req.to_grpc());
        payload.set_seq(payload.seq);
        payload.set_deps(protobuf::RepeatedField::from_vec(
            self.deps.iter().map(|dep| dep.to_grpc()).collect(),
        ));
        payload.set_instance(Instance::to_grpc(&self.instance));
        payload
    }
}

impl AcceptOKPayload {
    pub fn from_grpc(payload: &grpc::AcceptOKPayload) -> Self {
        AcceptOKPayload {
            write_req: WriteRequest::from_grpc(payload.get_command()),
            instance: Instance::from_grpc(payload.get_instance()),
        }
    }

    pub fn to_grpc(&self) -> grpc::AcceptOKPayload {
        let mut payload = grpc::AcceptOKPayload::new();
        payload.set_command(self.write_req.to_grpc());
        payload.set_instance(self.instance.to_grpc());
        payload
    }
}

impl Instance {
    pub fn from_grpc(instance: &grpc::Instance) -> Self {
        Instance {
            replica: instance.get_replica() as usize,
            slot: instance.get_slot() as usize,
        }
    }

    pub fn to_grpc(&self) -> grpc::Instance {
        let mut instance = grpc::Instance::new();
        instance.set_replica(self.replica as u32);
        instance.set_slot(self.slot as u32);
        instance
    }
}

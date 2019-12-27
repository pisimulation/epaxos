extern crate futures;
extern crate grpc;
extern crate sharedlib;

use grpc::ClientStub;
use sharedlib::epaxos::*;
use sharedlib::epaxos_grpc::*;
use std::sync::Arc;

fn main() {
    let grpc_client =
        Arc::new(grpc::Client::new_plain("127.0.0.1", 8080, Default::default()).unwrap());
    let client = EpaxosExternalClient::with_client(grpc_client);
    // let mut write_req = WriteRequest::new();
    // write_req.set_key("pi".to_owned());
    // write_req.set_value(3);
    // let write_resp = client.write(grpc::RequestOptions::new(), write_req);
    // println!("Client3 wrote {:?}", write_resp.wait());
    let mut read_req = ReadRequest::new();
    read_req.set_key("pi".to_owned());
    let read_resp = client.read(grpc::RequestOptions::new(), read_req);
    match read_resp.wait() {
        Err(e) => panic!("Client3 panic {:?}", e),
        Ok((_, value, _)) => println!("Client3 read {:?}", value),
    }
}

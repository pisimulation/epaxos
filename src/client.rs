extern crate futures;
extern crate grpc;
extern crate rayon;
extern crate sharedlib;

use grpc::ClientStub;
use rayon::prelude::*;
use sharedlib::epaxos_grpc::*;
use sharedlib::logic::{WriteRequest, REPLICA_PORT, VA};
use std::{sync::Arc, time::Instant};

fn main() {
    let write_req1 = WriteRequest {
        key: "pi".to_string(),
        value: 1,
    };
    let mut write_reqs = vec![write_req1.to_grpc(); 10];
    write_reqs.par_iter_mut().enumerate().for_each(|(i, req)| {
        let grpc_client =
            Arc::new(grpc::Client::new_plain(VA, REPLICA_PORT, Default::default()).unwrap());
        let client = EpaxosServiceClient::with_client(grpc_client);
        let start = Instant::now();
        let res = client.write(grpc::RequestOptions::new(), req.clone());
        match res.wait() {
            Err(e) => panic!("Write Failed: {}", e),
            Ok((_, _, _)) => {
                let duration = start.elapsed();
                println!("{} Commit Latency: {:?}", i, duration);
            }
        }
    });
}

extern crate futures;
extern crate grpc;
extern crate rayon;
extern crate sharedlib;

use grpc::ClientStub;
use rayon::prelude::*;
use sharedlib::epaxos_grpc::*;
use sharedlib::logic::{WriteRequest, EU, JP, REPLICA_PORT};
use std::{sync::Arc, time::Instant};

fn main() {
    let write_req1 = WriteRequest {
        key: "pi".to_string(),
        value: 1,
    };
    let write_req2 = WriteRequest {
        key: "pi".to_string(),
        value: 2,
    };
    let mut write_reqs = Vec::new();
    write_reqs.push((write_req1.to_grpc(), JP));
    //write_reqs.push((write_req2.to_grpc(), EU));
    write_reqs
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, (req, dst))| {
            let grpc_client =
                Arc::new(grpc::Client::new_plain(dst, REPLICA_PORT, Default::default()).unwrap());
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

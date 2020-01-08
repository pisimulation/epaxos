extern crate futures;
extern crate grpc;
extern crate rayon;
extern crate sharedlib;

use grpc::ClientStub;
use rayon::prelude::*;
use sharedlib::epaxos_grpc::*;
use sharedlib::logic::{WriteRequest, LOCALHOST, REPLICA_INTERNAL_PORTS};
use std::{sync::Arc, time::Instant};

fn main() {
    let write_req1 = WriteRequest {
        key: "pi".to_string(),
        value: 3,
    };
    let write_req2 = WriteRequest {
        key: "pi".to_string(),
        value: 4,
    };
    let mut write_reqs = Vec::new();
    write_reqs.push((write_req1.to_grpc(), REPLICA_INTERNAL_PORTS[1]));
    write_reqs.push((write_req2.to_grpc(), REPLICA_INTERNAL_PORTS[4]));
    write_reqs
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, (req, dst))| {
            //println!("{}", i);
            let grpc_client =
                Arc::new(grpc::Client::new_plain(LOCALHOST, *dst, Default::default()).unwrap());
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

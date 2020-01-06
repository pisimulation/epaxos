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
    let mut write_reqs = Vec::new();
    let write_req1 = WriteRequest {
        key: "pi".to_string(),
        value: 1,
    };
    write_reqs.push((write_req1.to_grpc(), 1));
    let write_req2 = WriteRequest {
        key: "pi".to_string(),
        value: 2,
    };
    write_reqs.push((write_req2.to_grpc(), 2));
    let write_req3 = WriteRequest {
        key: "pi".to_string(),
        value: 3,
    };
    write_reqs.push((write_req3.to_grpc(), 3));
    let write_req4 = WriteRequest {
        key: "pi".to_string(),
        value: 4,
    };
    write_reqs.push((write_req4.to_grpc(), 4));
    let write_req5 = WriteRequest {
        key: "pi".to_string(),
        value: 5,
    };
    write_reqs.push((write_req5.to_grpc(), 5));
    let write_req6 = WriteRequest {
        key: "pi".to_string(),
        value: 6,
    };
    write_reqs.push((write_req6.to_grpc(), 6));
    let write_req7 = WriteRequest {
        key: "pi".to_string(),
        value: 7,
    };
    write_reqs.push((write_req7.to_grpc(), 7));
    let write_req8 = WriteRequest {
        key: "pi".to_string(),
        value: 8,
    };
    write_reqs.push((write_req8.to_grpc(), 8));
    let write_req9 = WriteRequest {
        key: "pi".to_string(),
        value: 9,
    };
    write_reqs.push((write_req9.to_grpc(), 9));
    let write_req10 = WriteRequest {
        key: "pi".to_string(),
        value: 10,
    };
    write_reqs.push((write_req10.to_grpc(), 10));
    write_reqs.par_iter_mut().for_each(|(req, i)| {
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

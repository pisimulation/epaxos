extern crate futures;
extern crate grpc;
extern crate sharedlib;

use grpc::ClientStub;
use sharedlib::epaxos_grpc::*;
use sharedlib::logic::{WriteRequest, LOCALHOST};
use std::{sync::Arc, thread, time::Instant};

fn main() {
    // let grpc_client =
    //     Arc::new(grpc::Client::new_plain(LOCALHOST, 10000, Default::default()).unwrap());
    // // let client = EpaxosServiceClient::with_client(grpc_client);
    let mut write_reqs = Vec::new();
    let write_req1 = WriteRequest {
        key: "pi".to_string(),
        value: 3,
    };
    write_reqs.push(write_req1.to_grpc());
    let write_req2 = WriteRequest {
        key: "pi".to_string(),
        value: 4,
    };
    write_reqs.push(write_req2.to_grpc());
    let write_req3 = WriteRequest {
        key: "pi".to_string(),
        value: 5,
    };
    write_reqs.push(write_req3.to_grpc());
    let write_req4 = WriteRequest {
        key: "pi".to_string(),
        value: 6,
    };
    write_reqs.push(write_req4.to_grpc());
    let write_req5 = WriteRequest {
        key: "pi".to_string(),
        value: 7,
    };
    write_reqs.push(write_req5.to_grpc());
    let handle = thread::spawn(move || {
        for (i, req) in write_reqs.iter().enumerate() {
            // crossbeam_thread::scope(|s| {
            //     s.spawn(|_| {
            let grpc_client =
                Arc::new(grpc::Client::new_plain(LOCALHOST, 10000, Default::default()).unwrap());
            let client = EpaxosServiceClient::with_client(grpc_client);
            // let req = WriteRequest {
            //     key: "pi".to_string(),
            //     value: 3,
            // };
            let start = Instant::now();
            let res = client.write(grpc::RequestOptions::new(), req.clone());
            match res.wait() {
                Err(e) => panic!("Write Failed: {}", e),
                Ok((_, _, _)) => {
                    let duration = start.elapsed();
                    println!("{} Commit Latency: {:?}", i, duration);
                }
            }
            //     })
            //  })
            //  .unwrap();
        }
    });

    handle.join().unwrap();
}

// let mut read_req = ReadRequest::new();
// read_req.set_key("pi".to_owned());
// let read_resp = client.read(grpc::RequestOptions::new(), read_req);
// match read_resp.wait() {
//     Err(e) => panic!("Client panic {:?}", e),
//     Ok((_, value, _)) => println!("Client read {:?}", value),
// }

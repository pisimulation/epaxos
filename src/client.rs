extern crate futures;
extern crate grpc;
extern crate rayon;
extern crate sharedlib;

use grpc::ClientStub;
use rayon::prelude::*;
use sharedlib::epaxos_grpc::*;
use sharedlib::logic::{WriteRequest, REPLICA_ADDRESSES, REPLICA_PORT};
use std::{env, sync::Arc, time::Instant};

fn main() {
    let args: Vec<String> = env::args().collect();
    let id: u32 = args[1].parse().unwrap();
    let write_req1 = WriteRequest {
        key: "pi".to_string(),
        value: 1,
    };
    let mut write_reqs = Vec::new();
    write_reqs.push((write_req1.to_grpc(), id));
    write_reqs
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, (req, id))| {
            let grpc_client = Arc::new(
                grpc::Client::new_plain(
                    REPLICA_ADDRESSES[*id as usize],
                    REPLICA_PORT,
                    Default::default(),
                )
                .unwrap(),
            );
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

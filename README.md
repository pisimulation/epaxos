# A Pure Rust Implementation of Egalitarian Paxos

## Paper Summary
State machine replication increases fault tolerance of a distributed system but all replicas need to have an agreement on which commands to execute and in what order. The [Egalitarian Paxos paper](https://www.cs.princeton.edu/courses/archive/fall19/cos418/papers/epaxos.pdf) describes another flavor of Paxos, a distributed consensus algorithm. The paper was published at SOSP 2013 by Iulian Moraru, David G. Andersen, and Michael Kaminsky.

In the classic Paxos, the order of command execution is determined by a pre-ordered queue. When a replica receives a command from a client, it tries to become the command leader by taking a free slot in the queue. The command leader sends Prepare messages to at least a majority of the replicas to try to claim a free slot. Then, if there is no split brain, it proposes its command by sending Accept messages. Therefore, committing a command requires 2 RTTs or more in the case of split brain.

In Multi-Paxos, there exists a stable leader who owns all the slots. The client only sends commands to this leader. The leader skips the prepare phase as it already owns the whole queue and directly proposes the commands. Such protocol saves RTTs but puts high load on one replica and incurs latency when the leading replica fails.

Unlike the classic Paxos, EPaxos replicas do not contend for slots. To establish ordering constraints, each operation has dependencies and each replica communicates with each other its view of the dependencies. All the replicas use the dependencies to reach the same ordering. Unlike Multi-Paxos, EPaxos does not have a designated leader so the load is evenly distributed among all replicas. All replicas can become a command leader. As a result, the system has higher throughput, availability, and failure tolerance as there is no need to interrupt the system to carry out leader election. The client can also choose to communicate with the closest replica to save RTTs.

The basic protocol consists of the following stages.
1. Pre-Accept stage
* A replica that receives a command from the client becomes a leader for that command. It sends PreAccept messages to the fast-path quorum, F+floor(F + 12). F is the number of tolerated failures.
* A quorum member sends a PreAcceptOK message including its view of dependencies of that command.
* The command leader proceeds to the Commit stage if all quorum members agree on the dependencies. Otherwise, it runs the Paxos-Accept stage.

2. (slow path) Paxos-Accept stage
* The command leader unions all the dependencies and sends Accept messages to at least floor(N/2). N is the number of replicas.
* Upon receiving at least floor(N/2)AcceptOK messages, it runs the Commit stage.

3. (fast path) Commit stage
* The command leader updates its log, notifies all other replicas by sending Commit messages, and sends a commit to the client.
* Any replica that receives a Commit message updates its log.

A replica does not need to execute commands until it needs to, e.g. until it receives a read command. To execute a command, a replica builds a dependency graph of that command, finds the strongly connected components, sort them topologically, and in each component, execute them by their sequence number. Each replica independently uses the same execution algorithm to reach the same execution order. Figure 1 shows a simplified EPaxos consensus algorithm among 5 replicas.

![fig1](https://github.com/pisimulation/epaxos/blob/master/img/epaxos_msg_flow.png)
Figure 1 EPaxos message flow (Source: the Egalitarian Paxos paper)

## Implementation
We used Rust version 1.40.0 to implement EPaxos. The source code is available on GitHub repository. The replicas communicate using Rust implementation of gRPC. We use rayon in the client to send requests in parallel. We use crossbeam to support concurrency in the consensus process. The structure of the repository is as follows.

`epaxos/epaxos.proto` specifies the format of the messages for the cluster's internal communication and external client-server communication.

`epaxos/src/lib/converter.rs` converts between gRPC messages to EPaxos messages.

`epaxos/src/lib/logic.rs` is not aware of gRPC. It only handles the consensus logic.

`epaxos/src/server.rs` communicates with other replicas, responds to the client, and uses the logic library to run the consensus.

`epaxos/src/client.rs` sends read/write requests to a server.

We deploy EPaxos in a distributed datastore, therefore two operations interfere when they target the same key.

In the paper, a write operation does not need to be executed until there is a read that interferes with it. However, in our implementation, we decided to execute commands as soon as it is committed to reduce read latency.

We assume that communications between replicas are non-Byzantine.

## Evaluation
We deployed EPaxos state machine replicas on the free-tier micro instances of Amazon EC2 (1 CPU, 2.5 GHz, Intel Xeon Family, 1 GB memory, and medium-to-low network performance) using Amazon Linux AMI 64-bit x86. The replicas are distributed across 5 Amazon EC2 datacenters: Virginia, Northern California, Oregon, Japan, and Ireland. The client is run on a MacBook Pro (2.6 GHz, Intel Core i7, 16 GB memory) located in Thailand. Figure 2 shows deployment of all the communicating agents, the RTTs between a replica and its fast quorum members, and the RTTs between the client and the two closest replicas.

![fig1](https://github.com/pisimulation/epaxos/blob/master/img/epaxos_deploy.png)
Figure 2 Wide Area Replication

We aim to reproduce the result of commit latency in wide area replication. However, we do not have the Rust implementations of other flavors of Paxos. Therefore, we only compare performance of EPaxos with 0% interference and EPaxos with 100% interference.

Since the instances are much less powerful than the paper's evaluation setup, the time it takes to perform consensus is higher, though the trend remains the same. Figure 3 shows the median of commit latencies when (1) client requests target the same key (EPaxos 100%), and (2) client requests target different keys (EPaxos 0%). We evaluate the latency on group sizes of 3 (CA, VA, EU) and 5 (CA, OR, VA, EU, JP). A replica's quorum peers are replicas that are closest to it. In each experiment, the client sends 20 write requests to the two closest replicas (Japan and Ireland) and measures the commit latency, that is the time between a request is sent and a commit is received. Conflicting requests require an additional round trip to agree on dependencies so the latency is higher for EPaxos 100%. The overall latency for 5 replicas (F = 12) is higher than 3 replicas (F = 1) because the quorum size is larger so the latency is dominated by the furthest quorum peer.

![fig1](https://github.com/pisimulation/epaxos/blob/master/img/epaxos_commit_latency.png)
Figure 3 Median Commit Latency

## Conclusion
We successfully reproduced the basic Egalitarian Paxos without the execution algorithm. We enjoyed learning Rust and fighting with the borrow checker. Thanks to Rust's ownership concept, we get memory safety for free. Therefore, it is easier to get parallelism and concurrency right.

# Introduction
This project is a distributed key-value store built using Rust that leverages a gossip-based protocol for synchronizing data across multiple nodes. The system allows users to store, retrieve, and remove key-value pairs across different nodes in a cluster. It includes built-in caching mechanisms based on the Foyer and Moka caching libraries, providing an efficient way to handle large volumes of data with a scalable architecture.

Each node in the network can communicate with other nodes through the Gossip protocol. This enables the propagation of data updates and synchronization across the cluster without needing centralized coordination. The HTTP API exposes endpoints for adding, querying, and deleting key-value pairs.

# Key Features:
- Distributed Data Synchronization: Uses Gossip protocol for efficient data distribution across multiple nodes.
- In-Memory Caching: Supports in-memory caching using Foyer and Moka caches, providing fast data access.
- Scalable Design: Nodes can dynamically join the cluster, and the system ensures that new nodes receive up-to-date information from the existing ones.
- Simple HTTP Interface: Provides easy-to-use RESTful APIs for interacting with the distributed cache.

# Quick Start

You can start the system by running multiple nodes with different configurations. Here's how you can start a few nodes and interact with them:



```shell
# start node1
cargo run -- --name node1 --http-addr 0.0.0.0:3001 -g 0.0.0.0:4001

# start node2
cargo run -- --name node2 --http-addr 0.0.0.0:3002 -g 0.0.0.0:4002 --gossip-join-addr 0.0.0.0:4001

# start node3
cargo run -- --name node3 --http-addr 0.0.0.0:3003 -g 0.0.0.0:4003 --gossip-join-addr 0.0.0.0:4001

# node1 add
curl -X POST http://localhost:3001/add \
    -H "Content-Type: application/json" \
    -d '{"key": "hello", "value": "world"}'

# query
curl -X GET "http://localhost:3001/query?key=hello"
curl -X GET "http://localhost:3002/query?key=hello"
curl -X GET "http://localhost:3003/query?key=hello"

# node2 add
curl -X POST http://localhost:3002/add \
    -H "Content-Type: application/json" \
    -d '{"key": "node2", "value": "hello node2"}'
    
# query
curl -X GET "http://localhost:3001/query?key=node2"
curl -X GET "http://localhost:3002/query?key=node2"
curl -X GET "http://localhost:3003/query?key=node2"

# remove
curl -X DELETE http://localhost:3001/delete \
    -H "Content-Type: application/json" \
    -d '{"key": "hello"}'

# query
curl -X GET "http://localhost:3001/query?key=hello"
curl -X GET "http://localhost:3002/query?key=hello"
curl -X GET "http://localhost:3003/query?key=hello"
```

# Refer
- Moka cache: https://github.com/moka-rs/moka
- Foyer cache: https://github.com/foyer-rs/foyer
- BCache: https://github.com/iwanbk/bcache
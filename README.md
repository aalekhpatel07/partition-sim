# Network Partition Simulator


## Use cases
- To test the operation of a distributed protocol under unstable (but controlled) network conditions.
- To test the behaviour of a web-service under flaky network conditions. (Example: disabling/enabling network connectivity to the server).

## Architecture

Individual `node`s work as docker containers and form a cluster of test nodes. They register their address and ports with `Consul` when they're ready
to serve requests. The nodes also register an http healthcheck with Consul so that they can be monitored periodically.

The `supervisor` node is the orchestrator node that tweaks the `iptables` rules on the whole test cluster. The supervisor loads the state of the cluster
by querying Consul. Once the cluster under test is loaded by the supervisor (via `/api/v1/load_cluster`), the supervisor is ready to serve the Partition API.

The supervisor understands the following network-related mutations and queries:

1. `Partition` - `POST api/v1/partition/<source_id>/<target_id>`: Given ids of a source and a target node, configure the firewall on the target node such that all packets coming from the source node are dropped.
2. `Heal` - `POST api/v1/heal/<source_id>/<target_id>`: Given ids of a source and a target node, configure the firewall on the target node such that all packets coming from the source node are accepted.
3. `Restore` - `POST api/v1/restore`: Clear all the firewall rules across the cluster so that all nodes can communicate with each other.
4. `Rules` - `GET api/v1/rules/<target_id>`: Given id of a target node, list all the `INPUT` rules currently configured on the target node.

## Usage

Dockerize the system into a single process that will communicate with other docker containers whenever necessary. Ensure the system accepts `http` healthchecks at `/health`.

Consider using `docker/test-node.Dockerfile` as reference to structure your Dockerfile:

```Dockerfile
FROM rust:1.66 AS build
WORKDIR /app
RUN apt-get update -y && apt-get upgrade -y
RUN apt-get install curl python3-venv openssh-client openssh-server iptables sudo -y
COPY register_service.py /register_service.py
RUN chmod +x /register_service.py
RUN python3 -m venv /var/venv/node
RUN /var/venv/node/bin/python -m pip install requests
RUN mkdir -p /etc/ssh
COPY docker/sshd_config /etc/ssh/sshd_config

# Application related config.
...

# Make sure the `register_service.py` script gets called before your applications boots up.
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
```

`entrypoint.sh`

```sh
usermod --password $(echo "root" | openssl passwd -1 -stdin) root
service ssh restart
/var/venv/node/bin/python3 /register_service.py --name "test-node" --port 9000

# Now start your long running application that uses port 9000 to communicate with its peers.
...
```

The `docker/test-supervisor.Dockerfile` is to be used as is. Once the node's dockerfile is set up, start the `docker-compose` deployment:

```sh
docker-compose up -d
```

The `test-supervisor` container exposes an http api at port `3000` which can be used to control the network partitions in the test cluster.
The consul container is used solely for service discovery but if you're interested its UI can be accessed at port `8500` of the `consul` container.

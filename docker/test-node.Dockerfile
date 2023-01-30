FROM test-node-base AS test-node

FROM rust:1.66 AS build
WORKDIR /app
RUN apt-get update -y && apt-get upgrade -y
RUN apt-get install curl python3-venv openssh-client openssh-server iptables sudo -y
COPY --from=test-node /register_service.py /register_service.py
RUN chmod +x /register_service.py
RUN python3 -m venv /var/venv/node
RUN /var/venv/node/bin/python -m pip install requests
RUN mkdir -p /etc/ssh
COPY docker/sshd_config /etc/ssh/sshd_config

COPY src/ ./src/
COPY Cargo.toml ./
COPY Cargo.lock ./
RUN cargo install --path .

COPY docker/test-node-entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]

FROM rust:1.66
WORKDIR /app
RUN apt-get update -y && apt-get upgrade -y
RUN apt-get install curl dnsutils openssh-client openssh-server sshpass -y
COPY src/ src/
COPY Cargo.toml .
COPY Cargo.lock .
RUN cargo install --path .
RUN echo 'root' > /password.txt
RUN yes | ssh-keygen -t ed25519 -b 4096 -f ~/.ssh/id_ed25519 -N ''

ENTRYPOINT ["supervisor"]

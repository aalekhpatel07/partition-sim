#!/usr/bin/env sh

# adduser supervisor
# adduser supervisor sudo
usermod --password $(echo "root" | openssl passwd -1 -stdin) root

service ssh restart

/var/venv/node/bin/python3 /register_service.py --name "test-node" --port 9000
client --port 9000

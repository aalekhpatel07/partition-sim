#!/usr/bin/env python3

import uuid
import requests
import argparse
import socket
from pprint import pprint

def my_local_ip():
    return socket.gethostbyname(socket.getfqdn())


def parse_args():
    parser = argparse.ArgumentParser("Register a service with Consul")
    parser.add_argument(
        "--consul-base-url",
        type=str,
        default="http://consul:8500",
        help="Base URL (without trailing slash) for the Consul HTTP API"
    )
    parser.add_argument(
        "--name",
        type=str,
        required=True,
        help="Name of the service to register"
    )
    parser.add_argument(
        "--port",
        type=int,
        nargs="+",
        required=True,
        help="Ports for the service to register"
    )
    parser.add_argument(
        "--address",
        type=str,
        default=my_local_ip(),
        help="Address of the service to register"
    )
    return parser.parse_args()



def main():
    args = parse_args()
    for port in args.port:
        output = register_service(
            name=args.name,
            id=f"{args.name}-{str(uuid.uuid4())}",
            port=port,
            address=args.address,
            base_url=args.consul_base_url
        )
        pprint(output)


def register_service(
    base_url: str,
    name: str,
    id: str,
    port: int,
    address: str
):
    data = {
        "Name": name,
        "ID": id,
        "Port": port,
        "Address": address,
        "Meta": {
            "raft-infra-test-node": "true"
        }
    }
    pprint(data, indent=4)
    response = requests.put(
        f"{base_url}/v1/agent/service/register",
        json=data,
        headers={"Content-Type": "application/json"}
    )
    response.raise_for_status()
    return response


if __name__ == '__main__':
    main()

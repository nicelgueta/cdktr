import zmq
import time
import rustyrs
import random
context = zmq.Context()

PORT = 5561
HOST = "0.0.0.0"

def sync_to_subscriber(port: int):
    #  Socket to talk to server
    socket = context.socket(zmq.REP)
    socket.bind(f"tcp://{HOST}:{port}")

    print("Waiting for sync")
    socket.recv()
    print("Received sync") 
    socket.send(b"SYNC")

    
def start_pub_socket(i=False):
    #  Socket to talk to server
    print("Connecting to hello world server…")

    socket = context.socket(zmq.PUB)
    socket.bind(f"tcp://{HOST}:{PORT}")
    print(f"Running on tcp://{HOST}:{PORT}")

    # input("Ready?")
        
    gen = rustyrs.SlugGenerator(5)
    while True:
        if i:
            msg = input("Enter msg: ")
        else:
            msg = f"echo|{next(gen)}"
            print(f"Sending: {msg}")
            time.sleep(0.05)
        socket.send(bytes(msg, 'utf-8'))


def start_req_socket():
    #  Socket to talk to server
    print("Connecting to hello world server…")

    socket = context.socket(zmq.REQ)
    cli_port = PORT+1
    socket.connect(f"tcp://{HOST}:{cli_port}")
    print(f"Connected to tcp://{HOST}:{cli_port}")


    while True:
        msg = input("Enter message: ")
        socket.send(bytes(msg, 'utf-8'))
        message = socket.recv()
        print(f"Received reply: {message.decode('utf-8')}")

import sys
if len(sys.argv) > 1:
    match sys.argv[1]:
        case "pub":
            start_pub_socket(False)
        case "pubi":
            start_pub_socket(True)
        case "req":
            start_req_socket()
        case "rpub":
            socket = context.socket(zmq.REQ)
            cli_port = PORT+1
            socket.connect(f"tcp://{HOST}:{cli_port}")
            socket.send(b"TMRESTART")
            message = socket.recv()
            print(f"Received reply: {message.decode('utf-8')}")
            start_pub_socket()
        case _:
            print("Invalid argument")
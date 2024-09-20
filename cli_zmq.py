import zmq
import time
context = zmq.Context()

PORT = 5564
HOST = "0.0.0.0"

def sync_to_subscriber(port: int):
    #  Socket to talk to server
    socket = context.socket(zmq.REP)
    socket.bind(f"tcp://{HOST}:{port}")

    print("Waiting for sync")
    socket.recv()
    print("Received sync") 
    socket.send(b"SYNC")

def create_task():
    next_run = int(time.time()) + 5
    return (
        'CREATETASK|{"task_name": "echo hello","task_type": '
        '"PROCESS","command": "echo","args": "python generated task","cron": '
        f'"* * * * * *","next_run_timestamp": {next_run}' "}"
    )


def start_req_socket(principal_port):
    #  Socket to talk to server
    print("Connecting to hello world server…")

    socket = context.socket(zmq.REQ)
    socket.connect(f"tcp://{HOST}:{principal_port}")
    print(f"Connected to tcp://{HOST}:{principal_port}")

    print("Creating dummy task")
    task_msg = create_task()
    socket.send(bytes(task_msg, 'utf-8'))
    message = socket.recv()
    print(f"Received reply: {message.decode('utf-8')}")
    while True:
        msg = input("Enter message: ")
        socket.send(bytes(msg, 'utf-8'))
        message = socket.recv()
        print(f"Received reply: {message.decode('utf-8')}")

import sys
if len(sys.argv) > 1:
    match sys.argv[1]:
        case "req":
            principal_port = input("Enter principal port: ")
            start_req_socket(principal_port)
        case _:
            print("Invalid argument")


# EXETASKDEF|5562|PROCESS|ls
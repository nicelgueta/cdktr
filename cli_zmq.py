import zmq
import time
import os

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

def run_task():
    cmd = input("Enter task command: ")
    args = input("Enter args (pipe delimited)")
    zmq_str = f"RUNTASK|PROCESS|{cmd}|{args}"
    return zmq_str

def start_req_socket(principal_port):
    #  Socket to talk to server
    print("Connecting to hello world server…")

    socket = context.socket(zmq.REQ)
    socket.connect(f"tcp://{HOST}:{principal_port}")
    print(f"Connected to tcp://{HOST}:{principal_port}")

    while True:
        print("What do you want to do? ")
        print("1. Simulate a ZMQ event task being sent to the Principal for agent execution")
        # print("2. Simulate a ZMQ event task being sent to the Principal for agent execution")
        ans = input("Answer: ")
        match ans:
            case "1":
                msg = run_task()
            case _:
                msg = None
                print("Not a valid option")
        if msg:
            socket.send(bytes(msg, 'utf-8'))
            message = socket.recv()
            print(f"Received reply: {message.decode('utf-8')}")

if __name__ == "__main__":
    p_port = os.getenv("CDKTR_PRINCIPAL_PORT", int(input("Enter principal port number")))
import zmq
import time
import os
import pandas as pd

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


def run_task():
    cmd = input("Enter task id: ")
    zmq_str = f"RUNTASK|{cmd}"
    return zmq_str

def start_req_socket(principal_port):
    #  Socket to talk to server
    print("Connecting to hello world server…")

    socket = context.socket(zmq.REQ)
    socket.connect(f"tcp://{HOST}:{principal_port}")
    print(f"Connected to tcp://{HOST}:{principal_port}")

    while True:
        print("\nWhat do you want to do? ")
        print("1. Simulate a ZMQ event task being sent to the Principal for agent execution")
        print("2. Stream a load of run tasks with a given interval to the principal")
        print("3. List all tasks saved in db")
        ans = input("Answer: ")
        match ans:
            case "1":
                msg = run_task()
                if msg:
                    socket.send(bytes(msg, 'utf-8'))
                    message = socket.recv()
                    print(f"Received reply: {message.decode('utf-8')}")
            case "2":
                stream_interval_ms = int(input("Set interval in millis: "))
                task_n = 0
                while True:
                    socket.send(bytes(f"RUNTASK|ex-wf", 'utf-8'))
                    message = socket.recv()
                    print(f"Received reply: {message.decode('utf-8')}")
                    time.sleep(stream_interval_ms/1000)
                    task_n+=1
            case "3":
                    socket.send(bytes(f"LSWORKFLOWS", "utf-8"))
                    message = socket.recv()
                    print(f"Received reply: {message.decode('utf-8')}")

            case _:
                msg = None
                print("Not a valid option")

if __name__ == "__main__":
    import dotenv
    dotenv.load_dotenv()
    p_port = int(os.getenv("CDKTR_PRINCIPAL_PORT") or input("Enter principal port number: "))
    start_req_socket(p_port)
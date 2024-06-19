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

    
def start_pub_socket():
    #  Socket to talk to server
    print("Connecting to hello world server…")

    socket = context.socket(zmq.PUB)
    socket.connect(f"tcp://{HOST}:{PORT}")
    print(f"Running on tcp://{HOST}:{PORT}")

    # sync_to_subscriber(PORT+1)


    time.sleep(2)
    # for slug in rustyrs.generate_slugs(2, 10):
    #     time.sleep(random.random())
    #     socket.send(bytes(slug, 'utf-8'))c

    while True:
        msg = input("Enter message: ")
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

start_req_socket()
# start_pub_socket()
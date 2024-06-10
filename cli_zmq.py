import zmq
import time
import rustyrs
import random
context = zmq.Context()

PORT = 5561

def sync_to_subscriber(port: int):
    #  Socket to talk to server
    socket = context.socket(zmq.REP)
    socket.bind(f"tcp://0.0.0.0:{port}")

    print("Waiting for sync")
    socket.recv()
    print("Received sync") 
    socket.send(b"SYNC")

    

#  Socket to talk to server
print("Connecting to hello world server…")

socket = context.socket(zmq.PUB)
socket.bind(f"tcp://0.0.0.0:{PORT}")

# sync_to_subscriber(PORT+1)


time.sleep(2)
# for slug in rustyrs.generate_slugs(2, 10):
#     time.sleep(random.random())
#     socket.send(bytes(slug, 'utf-8'))c

while True:
    msg = input("Enter message: ")
    socket.send(bytes(msg, 'utf-8'))
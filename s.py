import time
import sys

def conducktor_print(s: str):
    print(s)
    sys.stdout.flush()

for i in  range(int(sys.argv[1])):
    conducktor_print(i)
    time.sleep(1)
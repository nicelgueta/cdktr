import time
import sys

def conducktor_print(s: str):
    print(s)
    try:
        sys.stdout.flush()
    except BrokenPipeError:
        # was executed from a fire and forget
        # process so just ignore
        pass

for i in  range(int(sys.argv[1])):
    conducktor_print(i)
    time.sleep(1)
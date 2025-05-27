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

id_, task_time = sys.argv[1:]
conducktor_print(f"Started task {id_} with task_time {task_time}")
for i in range(int(task_time)):
    conducktor_print(f"ckpt: {i} of {task_time}")
    time.sleep(1)
conducktor_print(f"Exiting python task {id_}")
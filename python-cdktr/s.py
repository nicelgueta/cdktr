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
time.sleep(int(task_time))
conducktor_print(f"Exiting python task {id_}")
# Conducktor

Super lightweight, distributed, terminal-based workflow orchestration engine for just about anything. Think about it like a TUI for oragnised cron and event-triggered jobs across different systems.
Executors are spawned as separate processes to run flows in parallel. When a flow is triggered, the scheduler will look for a machine with available executors to run the flow and send the flow to that instance to be executed.

## Main Features

1. TUI for interacting with current instance or different instances from other machines
1. Event listener to listen to incoming flow triggers
1. Scheduler - main event loop to poll databases for scheduled tasks, respond to TUI requests and action events from the event listener
1. Executor - a process spawned by a Scheduler to execute some flow


## Defining a flow

Flows can be defined in yaml:

`myflow.yaml`
```yaml

name: my-flow
schedule: "0 0 * * * *"
# or use an event trigger
eventTrigger: "my-event"
tasks:
    - name: First task
      command: echo hello
    - name: Second task
      command: echo world
    - name: Third task
      command: echo "hello world"
    - name: Fourth task
      command: echo "FAILED"
dag:
    # Simple linear flow
    - First task: Second task

    # Conditional flow
    - Second task?: Third task | Fourth task
```


## CLI Commands

```bash
# commands for master instance
$ conducktor init # Initialize a new conducktor project - master instance
$ conducktor ui # Start the TUI for the master instance
$ conducktor logs --flow [flow-name] --tail [n] # Get logs for a specific flow

# all below commands take the following options
$ conducktor \
    --host-port [hostname:PORT] # specify the hostname and port of the master instance
    [command] # the command to run


# Commands for scheduler instance
$ conducktor start --max-executors [max-executors] # Start the scheduler event loop
$ conducktor stop # Stop the scheduler

# Commands for executor instance
$ conducktor run [commmand] [args] # Run a command within a single Executor
$ conducktor trigger --payload [payload] [event-name]  # Trigger an event
```

## Settings
These can be set as environment variables, set in a local `conducktor.config.yaml` or passed as command line arguments - in ascending order of precedence.

- `CDKTR_HOME` - The directory where conducktor stores its configuration and state
- `CDKTR_MASTER_HOST` - The hostname of the master instance - needed for scheduler and executor instances not on the same machine
- `CDKTR_MASTER_PORT` - The port of the master instance ZMQ event listener
- `CDKTR_HOST` - The hostname of the current instance
- `CDKTR_PORT` - The port of the current instance ZMQ event listener
- `CDKTR_MAX_EXECUTORS` - The maximum number of executor threads to spawn for a given instance




## Current tasks
- 

## Issues
- need to refresh zmq subscriptions to check if the pub dropped for whatever reason
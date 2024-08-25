# CDKTR (conducktor)

Super lightweight, distributed, terminal-based workflow orchestration engine for just about anything - built on ZeroMQ.

Think about it like a TUI for organised cron and event-triggered jobs across different systems.
Executors are spawned as separate processes to run flows in parallel. When a flow is triggered, the scheduler will look for an agent with available executors to run the flow and send the flow to that instance to be executed.

## Main Features

1. TUI for interacting with current instance or different instances from other machines
1. Event listener to listen to incoming flow triggers
1. Scheduler - main event loop to poll databases for scheduled tasks, respond to TUI requests and action events from the event listener
1. Executor - a process spawned by a Scheduler to execute some flow


## tbd

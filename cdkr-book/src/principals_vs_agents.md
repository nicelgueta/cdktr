# Principals vs Agents

In the context of the CDKR, we distinguish between two types of entities: **principals** and **agents**. At a high-level, agents are responsible for executing tasks, while principals are responsible for orchestrating the execution of tasks. A principal may also be an agent, but will also be running the components responsible for orchestration and task management/distribution.

## Agents

Agents are CDKTR entities that are responsible for the execution of tasks. Agents are very lightweight and are designed to be easily scalable. They are composed of the following components:
- **TaskManager**: Responsible for executing tasks that are routed to it by its related `Principal` instance. The `TaskManager` spawns `Executor` instances to execute tasks. It also manages a `TaskManagerPub` component that listens for messages from its `Principal` instance to spawn tasks to be executed.
- **Executor**: an async task execution component spawned by the `TaskManager` to execute a single task.
- **Server**: a ZMQ REP component that listens for messages from its `Principal` instance for administrative tasks not-related to task execution.

```mermaid
graph LR
    B[Agent] --> C[TaskManager]
    C -->|Spawns| D[Executor]
    B --> E[Server]
```

## Principals

Principals are CDKTR entities that are responsible for the orchestration of tasks and act as the central point of control for the system. They are composed of the same components as an `Agent`, but also include additional components for task routing and scheduling, such as the following:

- **TaskRouter**: Responsible for routing tasks to the appropriate `Agent` instances based on their available resources and the task's requirements.
- **Publisher**: a ZMQ PUB component that publishes messages to a single TCP socket that all `Agent` instances are connected to.
- **Scheduler**: The main event loop that polls databases for scheduled tasks and sends them to the `TaskRouter` for routing.
- **Server**: An extension of an `Agent` a ZMQ REP implementation - also to provide a client/request API for interacting with the `Principal` instance itself from external systems. It supports all the same endpoints as a standard `Agent` instance however, so can be treated as a drop-in replacement for an `Agent` instance without having to change it's API or implementation.
- **EventListener**: a `TBD` component that listens for incoming flow triggers from external systems.

# Agents

Agents are the workhorses of the cdktr system. While the principal coordinates and schedules workflows, agents are responsible for the actual execution of work. Each agent is an autonomous process that can run on the same machine as the principal or on entirely different machines across a network, providing true distributed workflow execution.

## What Does an Agent Do?

At its core, an agent performs three primary functions:

1. **Work Acquisition**: Agents continuously poll the principal for available workflows using the `FetchWorkflow` API. When a workflow is available in the principal's task queue, an agent receives it and begins execution. This pull-based model ensures that agents only take on work when they have capacity to handle it.

2. **Workflow Execution**: Once assigned a workflow, the agent executes its tasks according to their dependency graph. The agent manages all aspects of task execution including launching executors, collecting output streams (stdout and stderr), and enforcing concurrency limits.

3. **Status Reporting**: Throughout the workflow lifecycle, agents send regular updates back to the principal about workflow and task states (PENDING, RUNNING, COMPLETED, FAILED, etc.). This keeps the principal's view of the system synchronized with reality.

Additionally, agents send periodic heartbeats to the principal every 5 seconds to signal they are still alive and ready for work. This heartbeat mechanism allows the principal to detect crashed agents and mark currently running workflows as CRASHED if connections to the agent are lost.

## The Internal Task Manager

The heart of every agent is its `TaskManager` component, which orchestrates the parallel execution of workflow tasks. When an agent receives a workflow from the principal, the TaskManager performs several sophisticated operations to execute it efficiently.

### DAG Construction and Topological Ordering

Workflows in cdktr are represented internally as a **Directed Acyclic Graph (DAG)**. Each workflow is converted into a DAG structure that maps task dependencies into graph nodes and edges. The DAG construction validates that there are no circular dependencies that would create deadlocks.

The key innovation here is that the DAG is **topologically sorted** when loaded into the agent's task tracker component. This topological sort identifies which tasks have no dependencies and can execute immediately—these become the "first tasks" loaded into a ready queue.

### Parallel Task Execution

The task manager maintains a ready queue of tasks that have had all their dependencies satisfied and are ready to execute. The aim of this design is that **tasks without dependencies on each other can execute in parallel**. When a task completes successfully, the task tracker:

1. Marks the task as successful
2. Examines its dependents in the DAG
3. Adds those dependents to the ready queue
4. Allows the next iteration of the execution loop to pull from the queue

This means if you have a workflow with three independent data ingestion tasks followed by a merge task, all three ingestion tasks will execute simultaneously, and only when all three complete will the merge task begin.

The agent respects its configured `max_concurrent_workflows` setting, ensuring it never attempts to run more workflows than it has resources for. Within each workflow, the same concurrency limits apply to individual task execution.

### Failure Handling and Cascading Skips

The task tracker implements intelligent failure handling. When a task fails:

1. The task is marked as failed
2. The tracker traverses the DAG to find all tasks that depend (directly or transitively) on the failed task
3. Those dependent tasks are automatically skipped and will not execute
4. Tasks with no dependency path to the failed task continue executing normally

This prevents wasted computation on tasks that cannot possibly succeed due to upstream failures, while still allowing independent branches of the workflow DAG to complete successfully.

## Resilience and Log Handling

One of cdktr's most robust features is its resilience to temporary network failures or principal unavailability. This is particularly critical for log handling, as tasks may generate significant output that needs to be captured even when the principal is unreachable.

### Buffered Log Publishing

Agents publish task logs using a log publisher component that implements a **local buffering queue**. When a task generates log output:

1. The log message is sent to the principal's log listener via ZeroMQ PUB socket
2. If the send fails (principal unreachable, network partition, etc.), the message is **automatically queued in a local buffer** within the agent
3. On the next log message, the publisher attempts to clear the buffered messages first
4. If the connection has been restored, buffered messages are sent before new ones

This ensures that **no log data is lost** even during temporary connection issues. Tasks continue executing and their output is preserved locally until the principal becomes reachable again.

### Graceful Degradation

When an agent loses connection to the principal during workflow execution, it doesn't immediately crash. Instead:

1. The workflow execution loop continues running any in-progress workflows
2. Status updates fail to send but the workflow continues executing
3. The agent waits for all running workflows to complete their execution
4. Only after all workflows have finished does the agent exit with an error

This "complete what you started" philosophy means that temporary principal failures don't cause unnecessary work loss. If a long-running data pipeline is 90% complete when the principal goes down, the agent will finish that last 10% rather than abandoning all progress.

### Reconnection Logic

The log publisher includes reconnection logic that recreates ZeroMQ socket connections when they fail. Combined with the local buffering, this means agents can ride out principal restarts or network blips without manual intervention.

## Agent Lifecycle

A typical agent lifecycle looks like this:

1. **Startup**: Agent creates its task manager with a unique instance ID
2. **Registration**: Agent registers with the principal
3. **Heartbeat**: Background task spawns to send heartbeats every 5 seconds
4. **Work Loop**: Agent enters its main workflow execution loop, continuously polling for work
5. **Workflow Acquisition**: When work is available, agent receives a workflow
6. **Execution**: Workflow tasks execute in parallel according to DAG dependencies
7. **Completion**: Final workflow status sent to principal, workflow counter decremented
8. **Repeat**: Agent returns to polling for more work

This continues indefinitely until the agent is explicitly shut down or loses connection to the principal in an unrecoverable way.

## Configuration

Agents are configured primarily through the `CDKTR_AGENT_MAX_CONCURRENCY` environment variable (default: 5), which controls how many workflows an agent can execute simultaneously. Higher values allow more parallelism but consume more system resources.

Agents also respect the general ZeroMQ configuration settings like `CDKTR_RETRY_ATTEMPTS` and `CDKTR_DEFAULT_ZMQ_TIMEOUT_MS` when communicating with the principal.

## Horizontal Scaling

The beauty of cdktr's agent architecture is how trivially it scales horizontally. To handle more workflow throughput:

1. Launch additional agent processes on the same or different machines
2. Point them at the same principal using `CDKTR_PRINCIPAL_HOST` and `CDKTR_PRINCIPAL_PORT`
3. Each agent automatically registers and begins polling for work

Agents only request work from the principal when they have available capacity (i.e., they haven't hit their concurrency limits). This self-regulating behavior means work naturally flows to available agents without requiring complex load balancing logic in the principal.

This means you can scale from a single-machine development setup to a distributed production cluster without changing a single line of configuration—just launch more agents.

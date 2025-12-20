# Logs & Database

The logging and data persistence system in cdktr is designed to provide comprehensive visibility into workflow execution while maintaining performance and reliability. This architecture enables both real-time monitoring and historical analysis through a combination of streaming and storage mechanisms.

## Log Streaming Architecture

At the heart of cdktr's observability is a publish-subscribe logging system built on ZeroMQ. This architecture creates a real-time stream of execution information that flows from agents executing tasks, through the principal coordinator, and out to any interested consumers.

### How Agents Publish Logs

When an agent executes tasks, it doesn't write logs to local files or directly to a database. Instead, each agent maintains a log publisher that sends messages to the principal over a dedicated ZeroMQ connection. This publisher uses a PUSH socket that connects to the principal's log manager, which listens on a PULL socket.

Every log message contains rich context about the execution:
- The workflow identifier and name
- The specific workflow instance being executed
- The task name and its unique execution identifier
- A timestamp marking when the event occurred
- The log level (INFO, WARN, ERROR, etc.)
- The actual log payload containing the message

This structured approach ensures that every log line can be traced back to its exact origin in the execution tree. When you're troubleshooting a failed task, you can immediately see which workflow instance it belonged to and when it occurred.

### The Principal's Log Manager

The principal runs a dedicated log manager service that acts as a central hub for all logging activity. This service receives log messages from all agents via its PULL socket and immediately republishes them through a PUB socket. This publish-subscribe pattern creates a broadcast mechanism where multiple consumers can receive the same log stream simultaneously.

The log manager doesn't process or filter the messages—it simply ensures they flow from agents to subscribers. This design keeps the critical path lightweight and fast. The manager operates on two ports configured via environment variables: one for receiving logs from agents (`CDKTR_LOGS_LISTENING_PORT`) and another for publishing to subscribers (`CDKTR_LOGS_PUBLISHING_PORT`).

### Real-Time Log Consumption

Any component can subscribe to the log stream by connecting to the principal's publishing port. The Terminal User Interface (TUI), for example, leverages this exact mechanism to provide live log tailing. When you select a workflow in the TUI and watch its logs scroll by, you're seeing the same messages that agents are publishing in real-time.

Subscribers can filter logs by workflow ID, allowing them to see only the logs relevant to specific workflows. This topic-based subscription means you're not overwhelmed with logs from every workflow running in your system—you see exactly what you choose to monitor.

The streaming nature of this system provides immediate feedback during execution. There's no delay waiting for logs to be written to disk, no polling interval to wait for—you see task output as it happens.

## Log Persistence with DuckDB

While streaming provides real-time visibility, you also need the ability to query historical execution data. This is where DuckDB enters the picture, providing a lightweight yet powerful analytical database for storing all workflow execution information.

### Why DuckDB?

DuckDB is an embedded analytical database—think of it as SQLite but optimized for analytical queries rather than transactional workloads. For cdktr's use case, this is ideal. You get the simplicity of an embedded database (no separate server to manage) combined with excellent performance for querying large volumes of log data.

The database can run in-memory for development and testing, or persist to disk for production deployments. This flexibility allows you to choose the appropriate storage strategy for your environment.

DuckDB is just great! Given your cdktr DB is just a single file, you can even open it directly with DuckDB's CLI or connect to it from Python or R for custom analysis.

### The Log Persistence Pipeline

The principal runs a dedicated log persister service that subscribes to the same log stream as other consumers. This service receives logs from the pub/sub system and batches them for efficient database insertion. Rather than writing each log individually, the persister accumulates messages in an asynchronous queue and flushes them to the database every 30 seconds.

This batching strategy significantly improves write performance. Database insertions are one of the more expensive operations, and batching reduces the overhead by consolidating many individual writes into a single efficient bulk operation.

If a database write fails—perhaps due to disk space issues or connection problems—the persister retains the failed batch in its queue and attempts to write it again on the next interval. This resilience ensures logs aren't lost even when the database experiences temporary issues.

### The Logstore Schema

All log messages are stored in the `logstore` table with a straightforward schema that mirrors the log message structure:
- **workflow_id**: The identifier for the workflow definition
- **workflow_name**: The human-readable workflow name
- **workflow_instance_id**: The unique identifier for this execution instance
- **task_name**: The name of the task that generated the log
- **task_instance_id**: The unique identifier for this task execution
- **timestamp_ms**: Milliseconds since epoch for precise time ordering
- **level**: The log level (INFO, WARN, ERROR, etc.)
- **payload**: The actual log message content

This schema enables powerful queries. You can retrieve all logs for a specific workflow instance, find all ERROR-level logs across all executions, or analyze task performance by examining timestamp patterns. The database provides a `QueryLogs` API that accepts time ranges, workflow filters, and instance filters to retrieve exactly the logs you need.

## Workflow Execution State

Beyond just log messages, cdktr persists all workflow execution state in the database. This creates a complete audit trail of everything that happens in your system.

### Insert-Only Architecture

A critical design decision is that all execution state is stored in insert-only tables. Nothing is ever updated or deleted. When a workflow starts, a record is inserted. When it completes, another record is inserted with a new timestamp. The same applies to individual tasks—each status change creates a new row.

This approach provides several advantages:
1. **Complete History**: You can reconstruct the entire timeline of any workflow execution by querying its status records chronologically
2. **Audit Trail**: Nothing is ever lost or overwritten, providing perfect auditability
3. **Performance**: Append-only operations are faster than updates, especially in analytical databases like DuckDB
4. **Simplicity**: No need for complex update logic or handling concurrent updates

### Workflow Run Status

The `workflow_run_status` table tracks workflow-level state changes:
- **workflow_id**: The workflow definition identifier
- **workflow_instance_id**: The unique instance identifier
- **status**: One of PENDING, RUNNING, WAITING, COMPLETED, FAILED, or CRASHED
- **timestamp_ms**: When this status change occurred

When an agent starts executing a workflow, it inserts a RUNNING status record. When the workflow completes (successfully or not), another record is inserted with the appropriate final status. By querying the most recent status for a workflow instance, you can determine its current state. By querying all statuses, you can see its complete execution timeline.

### Task Run Status

Similarly, the `task_run_status` table maintains the execution history for individual tasks:
- **task_id**: The task identifier from the workflow definition
- **task_instance_id**: The unique identifier for this task execution
- **workflow_instance_id**: The workflow instance this task belongs to
- **status**: The task's status (PENDING, RUNNING, COMPLETED, FAILED, etc.)
- **timestamp_ms**: When this status change occurred

This granular tracking means you can analyze task-level behavior. Which tasks fail most often? How long does a particular task typically run? Which tasks are bottlenecks? All these questions can be answered by querying the task status history.

### Querying Execution History

The principal provides API endpoints for querying this execution data. You can retrieve recent workflow statuses to see what's currently running or recently completed. The log query endpoint allows time-based and filter-based retrieval of both logs and status records.

The TUI and CLI tools leverage these APIs to present execution information to users. When you see a workflow's status in the TUI or query logs with the CLI, you're seeing data pulled directly from these DuckDB tables.

## Benefits of This Architecture

The combination of real-time streaming and persistent storage provides the best of both worlds:

**Immediate Visibility**: The pub/sub streaming means you see what's happening right now, with no delay. This is essential for monitoring long-running workflows and troubleshooting issues as they occur.

**Historical Analysis**: The DuckDB storage enables sophisticated queries over historical data. You can identify patterns, generate reports, and understand long-term trends in your workflow execution.

**Decoupled Consumers**: Because logging uses pub/sub, new consumers can be added without impacting agents or the principal. Want to send logs to an external monitoring system? Just add a new subscriber.

**Resilience**: Agents buffer logs locally if they can't reach the principal, the persister retries failed database writes, and the streaming continues even if persistence fails. The system is designed to preserve observability even when components experience issues.

**Scalability**: The streaming architecture scales horizontally—more agents just means more log publishers, and subscribers receive all messages regardless of how many agents are active. The database scales vertically with DuckDB's efficient analytical engine handling large query workloads.

This logging and persistence architecture ensures you always have the information you need to understand, monitor, and troubleshoot your distributed workflow executions.

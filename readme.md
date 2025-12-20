# CDKTR (conducktor)

![CDKTR Terminal UI](cdkr-book/src/assets/images/tui-main.png)

A lightweight, distributed workflow orchestration system designed for reliability and simplicity. Define your workflows in YAML, schedule them with cron expressions or trigger them from custom events, and monitor everything through a beautiful terminal interface.

**[📖 Read the full documentation](#)** _(coming soon)_

## What is CDKTR?

CDKTR is a distributed task orchestration system that brings the power of modern workflow engines to the terminal. It's built for developers and operators who need to coordinate complex task dependencies across multiple machines without the overhead of heavyweight orchestration platforms.

Think Airflow or Temporal, but designed for simplicity and terminal-first workflows. Define your pipelines in version-controlled YAML files, let CDKTR handle the scheduling and execution, and watch everything happen in real-time through the TUI.

## Key Features

### 🏗️ **Distributed Architecture**
- **Principal-Agent Model**: A central coordinator (principal) distributes work to multiple execution nodes (agents)
- **Horizontal Scaling**: Add more agents to increase capacity—they automatically register and start accepting work
- **Pull-Based Design**: Agents request work when ready, preventing overload and enabling natural load balancing
- **Resilience**: Agents buffer logs and gracefully handle principal disconnections, workflows automatically fail over when agents crash

### 📋 **Workflow as Code**
- **YAML Definitions**: Define workflows in simple, readable YAML files stored in Git
- **DAG Execution**: Tasks declare dependencies, and CDKTR automatically determines optimal parallel execution
- **Hot Reload**: Changes to workflow files are detected automatically (every 60 seconds by default)—no restarts needed
- **Source Control**: Workflows are just files, so you get Git history, pull requests, and GitOps deployment patterns for free

### 🚀 **Flexible Task Execution**
- **Subprocess Tasks**: Run any shell command, script, or executable
- **Python Tasks**: First-class support for Python with `uv` for lightning-fast dependency management
- **Parallel Execution**: Independent tasks run simultaneously across available agents
- **Failure Handling**: Failed tasks automatically cascade to skip dependent tasks

### 📡 **Event-Driven Scheduling**
- **Cron Scheduling**: Standard cron expressions for time-based workflows
- **Custom Event Listeners**: Build event sources in Rust or Python—file watchers, webhooks, message queues, database triggers, anything
- **Extensible**: The scheduler itself is just an event listener, making the entire system pluggable

### 📊 **Observability & Persistence**
- **Real-Time Logs**: ZeroMQ pub/sub streams all task output in real-time to any subscriber
- **DuckDB Storage**: All logs and execution state persisted in an embedded analytical database
- **Insert-Only Audit Trail**: Complete execution history with no updates or deletes—perfect auditability
- **Queryable History**: Time-range queries, workflow filters, task-level analysis

### 💻 **Terminal User Interface**
- **Live Monitoring**: Watch workflows execute in real-time with streaming logs
- **Workflow Management**: Browse, trigger, and monitor workflows from the terminal
- **Agent Visibility**: See all registered agents, their capacity, and current workloads
- **Multi-Instance**: Connect to different principals from the same TUI—manage multiple environments

## Quick Start

```bash
# Start the principal and an agent
cdktr start principal --with-agent

# Or run them separately
cdktr start principal
cdktr start agent

# Open the TUI
cdktr ui

# Trigger a workflow
cdktr task run my-workflow

# Query logs
cdktr logs query --workflow my-workflow --last 1h
```

## Architecture Overview

CDKTR's architecture is built around a few core components:

- **Principal**: The central coordinator that manages workflow definitions, schedules executions, distributes work to agents, and maintains execution state in DuckDB
- **Agents**: Execution nodes that request workflows, run tasks in parallel using DAG-based topological sorting, and stream logs back to the principal
- **Event Listeners**: Pluggable event sources that trigger workflows—the built-in scheduler handles cron expressions, and you can add custom listeners for any event source
- **Log Manager**: A ZeroMQ pub/sub hub that broadcasts all task output in real-time to the TUI, CLI tools, and persistence layer
- **Database**: DuckDB stores all logs and execution state in insert-only tables for complete audit trails and historical analysis

All communication happens over ZeroMQ with REQ/REP for commands and PUB/SUB for logs, making the system fast, lightweight, and easy to distribute across machines.

## Why CDKTR?

**Simple**: YAML workflow definitions, standard cron expressions, plain Python or shell scripts for tasks. No complex DSLs or proprietary languages.

**Lightweight**: Built in Rust for minimal resource usage. The entire system runs comfortably on modest hardware. DuckDB provides powerful analytics without external database servers.

**Terminal-First**: Built for operators who live in the terminal. The TUI provides everything you need without leaving your workflow.

**Version Control Native**: Workflows are just YAML files—use Git branches for testing, pull requests for reviews, and standard CI/CD for deployments.

**Observable**: Real-time log streaming means you see task output as it happens. Complete execution history means you can always answer "what happened?"

**Extensible**: Build custom event listeners in Rust or Python. The event-driven architecture makes integration straightforward.

## Documentation

For comprehensive guides covering installation, configuration, architecture deep-dives, and advanced usage:

**[📖 Full Documentation](#)** _(coming soon)_

## Development Status

CDKTR is under active development. Core features are functional:
- ✅ Distributed principal-agent architecture
- ✅ YAML workflow definitions with DAG execution
- ✅ Subprocess and Python task types
- ✅ Cron-based scheduling
- ✅ Real-time log streaming
- ✅ DuckDB persistence
- ✅ Terminal UI for monitoring and management
- ✅ Custom event listener support

Upcoming features and improvements are tracked in the issue tracker.

## License

See [LICENSE](LICENSE) for details.

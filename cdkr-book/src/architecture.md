# Architecture & Core Concepts

This chapter explains the fundamental architecture of cdktr and the core concepts you need to understand to effectively use the system.

## Overview

cdktr is a distributed workflow orchestration system built on a principal-agent architecture. It is designed to be lightweight and scalable, using source-controllable YAML files for workflow definitions, ZeroMQ for fast, reliable communication between components, and DuckDB for storing logs and execution history with powerful query capabilities.

## What's Covered

This section explores the key components and patterns that make cdktr work:

- **[System Overview](./architecture/overview.md)**: The big picture—how components fit together in a distributed pull-based architecture
- **[Principal](./architecture/principal.md)**: The central coordinator managing workflows, scheduling, work distribution, and persistent state
- **[Agents](./architecture/agents.md)**: Execution nodes that run tasks in parallel using DAG-based topological sorting
- **[Event Listeners & Scheduler](./architecture/events-scheduler.md)**: The event-driven architecture powering cron scheduling and custom event sources
- **[Workflows & Tasks](./architecture/workflows-tasks.md)**: YAML workflow definitions, task types (Subprocess and UvPython), and dependency execution
- **[Logs & Database](./architecture/logs-database.md)**: Real-time ZeroMQ log streaming and DuckDB persistence with insert-only audit trails
- **[Examples](./architecture/workflow-examples.md)**: Practical workflow examples demonstrating common patterns

## Key Design Principles

1. **Simplicity**: Single binary deployment with no external dependencies
2. **Performance**: Built in Rust for speed and safety
3. **Distributed**: Scale horizontally by adding more agents
4. **Observable**: TUI for real-time monitoring without a web UI
5. **Flexible**: ZeroMQ API allows integration with any language

Let's dive into each concept in detail.

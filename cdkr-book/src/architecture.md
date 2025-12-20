# Architecture & Core Concepts

This chapter explains the fundamental architecture of cdktr and the core concepts you need to understand to effectively use the system.

## Overview

cdktr is a distributed workflow orchestration system built on a principal-agent architecture. It is designed to be completely stateless, using source-controllable yaml files for workflow definitions, ZeroMQ for fast, reliable communication between components and stores logs, workflow and task history in a DuckDB database for fast analytics and querying.

The key concepts covered in this section are:

- **System Architecture**: How the various components fit together
- **Principals vs Agents**: The distributed computing model
- **Workflows & Tasks**: How work is defined and executed
- **ZeroMQ Communication**: The messaging layer that powers cdktr

## Key Design Principles

1. **Simplicity**: Single binary deployment with no external dependencies
2. **Performance**: Built in Rust for speed and safety
3. **Distributed**: Scale horizontally by adding more agents
4. **Observable**: TUI for real-time monitoring without a web UI
5. **Flexible**: ZeroMQ API allows integration with any language

Let's dive into each concept in detail.

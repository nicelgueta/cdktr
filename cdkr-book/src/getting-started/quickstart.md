# Quick Start Guide

This guide will walk you through setting up a basic cdktr system with a principal and agent, and running your first workflow.

## Step 1: Initialise a Project

Create a new directory for your cdktr project:

```bash
mkdir my-cdktr-project
cd my-cdktr-project
cdktr init
```

This creates a basic project structure with an example workflow in the `workflows/` directory.

## Step 2: Start a Principal

The principal is the central coordinator that manages workflow scheduling and task distribution. Start it in one terminal:

```bash
cdktr start principal
```

By default, the principal starts on port 5561. You can specify a different port:

```bash
cdktr start principal -p 5570
```

For light, single-machine setups you can invoke an agent process from the main principal process:
```bash
cdktr start principal --with-agent
```

You should see logs indicating that the principal has started successfully:

```
[INFO] Principal server started on 127.0.0.1:5555
[INFO] Scheduler found X workflows with active schedules
```

## Step 3: Start an Agent

Agents execute the actual workflow tasks. To start an agent in a new ternminal, run:

```bash
cdktr start agent
```

The agent will automatically connect to the principal running on the default port. If your principal is on a different port or host:

```bash
CDKTR_PRINCIPAL_HOST=127.0.0.1 CDKTR_PRINCIPAL_PORT=5570 cdktr start agent
```

You can specify the maximum number of concurrent workflows an agent can handle:

```bash
cdktr start agent --max-concurrent 5
```

## Step 4: Open the TUI

Now that you have a principal and agent running, open the TUI to monitor and manage workflows. The TUI provides a user-friendly interface to interact with your cdktr setup and can be started with:

```bash
cdktr ui
```

The TUI will connect to your principal and display:
- Available workflows
- Workflow execution status
- Registered agents
- Recent activity


## What's Next?

You now have a basic cdktr setup running! Here's what to explore next:

- Learn about [Basic CLI Commands](./cli-basics.md) for managing your setup
- Create [Your First Workflow](./first-workflow.md) from scratch
- Understand the [Architecture & Core Concepts](../architecture.md)

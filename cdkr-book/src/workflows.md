# Workflow Definition

Workflows are YAML files defining tasks, schedules, and dependencies.

## Basic Structure

```yaml
name: My Workflow
description: What it does (optional)
cron: "0 0 2 * * *"  # Schedule (optional)
tasks:
  task1:
    name: First Task
    config:
      !Subprocess
      cmd: echo
      args: ["Hello"]
  task2:
    name: Second Task
    depends: ["task1"]  # Wait for task1
    config:
      !Subprocess
      cmd: echo
      args: ["World"]
```

## Key Concepts

- **YAML Structure**: Workflow metadata and task definitions
- **Task Config**: Subprocess execution with commands and args
- **Scheduling**: Cron expressions for automated execution
- **Dependencies**: DAG-based task ordering with `depends` field
- **Examples**: Real workflows demonstrating features

# Task Configuration

cdktr supports two task types: **Subprocess** and **UvPython**.

## Subprocess Tasks

Run any command as a subprocess.

```yaml
config:
  !Subprocess
  cmd: <command>     # Required: executable in PATH or absolute path
  args:              # Optional: command arguments as list
    - <arg1>
    - <arg2>
```

**Examples:**

```yaml
# Python script
config:
  !Subprocess
  cmd: python
  args: ["script.py", "--input", "data.csv"]

# Shell script
config:
  !Subprocess
  cmd: bash
  args: ["./backup.sh"]

# HTTP request
config:
  !Subprocess
  cmd: curl
  args: ["-X", "POST", "https://api.example.com/webhook"]

# Database query
config:
  !Subprocess
  cmd: psql
  args: ["-h", "localhost", "-c", "SELECT COUNT(*) FROM orders;"]
```

## UvPython Tasks

Run Python scripts with automatic dependency management using [uv](https://docs.astral.sh/uv/).

```yaml
config:
  !UvPython
  script_path: <path>           # Required: path to Python script
  packages:                     # Optional: dependencies to install
    - package>=version
  is_uv_project: <bool>         # Optional: true if script is in uv project (default: false)
  working_directory: <path>     # Optional: execution directory
  uv_path: <path>               # Optional: custom uv executable path
```

### Standalone Script with Dependencies

```yaml
config:
  !UvPython
  script_path: ./process_data.py
  packages:
    - pandas>=2.3.1,<3.0.0
    - requests>=2.31.0
  working_directory: ./scripts
```

### uv Project

For scripts with inline dependencies or `pyproject.toml`:

```yaml
config:
  !UvPython
  script_path: ./analysis.py
  is_uv_project: true
  working_directory: ./my-project
```

### Inline Dependencies (PEP 723)

Your Python script can declare dependencies inline:

```python
# /// script
# dependencies = [
#   "pandas>=2.3.1,<3.0.0",
#   "httpx",
# ]
# ///

import pandas as pd
import httpx

# Your code here
```

Then use:

```yaml
config:
  !UvPython
  script_path: ./script.py
  is_uv_project: true
```

## Task Execution

### Working Directory
Tasks execute in agent's working directory unless `working_directory` is specified.

### Environment Variables
Tasks inherit agent's environment variables.

### Exit Codes
- **0**: Success
- **Non-zero**: Failure (workflow fails, dependents skip)

```yaml
config:
  !Subprocess
  cmd: psql
  args:
    - -h
    - localhost
    - -U
    - user
    - -d
    - mydb
    - -c
    - "SELECT COUNT(*) FROM orders;"
```

### Docker Commands

```yaml
config:
  !Subprocess
  cmd: docker
  args:
    - run
    - --rm
    - -v
    - /data:/data
    - my-image:latest
    - python
    - process.py
```

## Execution Context

### Working Directory

Tasks execute in the agent's current working directory.

### Environment Variables

Tasks inherit the agent's environment variables:

```bash
# Start agent with custom env vars
export DATABASE_URL=postgresql://localhost/mydb
export API_KEY=secret123
cdktr start agent
```

These are available to all tasks executed by that agent.

### Standard Streams

- **stdout**: Captured and logged to database
- **stderr**: Captured and logged to database
- **stdin**: Not supported (tasks run non-interactively)

### Exit Codes

- **0**: Task succeeded
- **Non-zero**: Task failed (workflow fails, dependents not executed)

## Best Practices

1. **Use Absolute Paths**: For scripts in specific locations
2. **Check Dependencies**: Ensure commands are available on agent machines
3. **Handle Errors**: Make scripts exit with proper codes
4. **Keep Tasks Focused**: One responsibility per task
5. **Log Appropriately**: Output to stdout/stderr for debugging

## Future Task Types

Planned task types for future releases:

- **Python**: Native Python function execution
- **SQL**: Direct database query execution
- **HTTP**: HTTP request tasks without curl
- **Email**: Email notification tasks

## Examples

### Complete Task Example

```yaml
backup_database:
  name: Backup Production Database
  description: Creates a timestamped backup of the production database
  config:
    !Subprocess
    cmd: pg_dump
    args:
      - -h
      - prod-db.example.com
      - -U
      - backup_user
      - -d
      - production
      - -f
      - /backups/prod_$(date +%Y%m%d).sql
```

### Multi-Step Process

```yaml
tasks:
  download:
    name: Download Dataset
    config:
      !Subprocess
      cmd: wget
      args:
        - -O
        - /tmp/dataset.zip
        - https://example.com/dataset.zip

  extract:
    name: Extract Files
    depends: ["download"]
    config:
      !Subprocess
      cmd: unzip
      args:
        - /tmp/dataset.zip
        - -d
        - /data/

  process:
    name: Process Data
    depends: ["extract"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/process.py
        - /data/
```

## Next Steps

- Learn about [Scheduling](./scheduling.md) workflows
- Understand [Task Dependencies](./dependencies.md)
- See more [Workflow Examples](./examples.md)

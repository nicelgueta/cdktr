# CLI Reference

The cdktr command-line interface provides commands for managing principals, agents, workflows, and logs.

## Command Structure

```bash
cdktr <command> [subcommand] [options]
```

## Available Commands

### ui
Launch the Terminal User Interface.

```bash
cdktr ui [--principal-host HOST] [--principal-port PORT]
```

See [TUI Chapter](./tui.md) for details.

### start
Start a principal or agent instance.

```bash
cdktr start <principal|agent> [OPTIONS]
```

See [Start Commands](./cli/start.md) for details.

### task
Manage and trigger workflows.

```bash
cdktr task <run|list> [OPTIONS]
```

See [Task Commands](./cli/task.md) for details.

### logs
Query execution logs.

```bash
cdktr logs query [OPTIONS]
```

See [Logs Commands](./cli/logs.md) for details.

### init
Initialize a new cdktr project.

```bash
cdktr init [PATH]
```

See [Init Command](./cli/init.md) for details.

## Global Options

### --help, -h
Show help information.

```bash
cdktr --help
cdktr start --help
cdktr task run --help
```

### --version, -V
Show version information.

```bash
cdktr --version
```

## Environment Variables

Many CLI options can be set via environment variables:

- `CDKTR_PRINCIPAL_HOST` - Default principal host
- `CDKTR_PRINCIPAL_PORT` - Default principal port
- `CDKTR_LOG_LEVEL` - Log verbosity (DEBUG, INFO, WARN, ERROR)
- `CDKTR_WORKFLOW_DIR` - Workflow directory path
- `CDKTR_DB_PATH` - Database file path

## Examples

### Start a complete setup
```bash
# Terminal 1: Start principal
cdktr start principal

# Terminal 2: Start agent
cdktr start agent

# Terminal 3: Open TUI
cdktr ui
```

### Trigger a workflow
```bash
cdktr task run my-workflow
```

### Query logs
```bash
cdktr logs query --workflow my-workflow --status FAILED
```

## Next Steps

- [Task Commands](./cli/task.md)
- [Start Commands](./cli/start.md)
- [Logs Commands](./cli/logs.md)
- [Init Command](./cli/init.md)

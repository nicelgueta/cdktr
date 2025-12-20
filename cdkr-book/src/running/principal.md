# Starting a Principal

## Basic Command

```bash
cdktr start principal
```

Defaults: `127.0.0.1:5555`

## Options

**--port, -p**: Server port (default: 5555)
```bash
cdktr start principal --port 5570
```

**--host, -h**: Bind address (default: 127.0.0.1)
```bash
cdktr start principal --host 0.0.0.0  # All interfaces
```

**--no-scheduler**: Skip scheduler component
```bash
cdktr start principal --no-scheduler
```

## Common Setups

**Local development:**
```bash
cdktr start principal
```

**Production (network accessible):**
```bash
cdktr start principal --host 0.0.0.0 --port 5555
```

**Manual triggers only:**
```bash
cdktr start principal --no-scheduler
```

## Startup Process

1. Initialize DuckDB database
2. Load workflows from `CDKTR_WORKFLOW_DIR`
3. Start ZeroMQ server
4. Start scheduler (unless `--no-scheduler`)
5. Start heartbeat monitor

Stop with `Ctrl+C`.
```

## Next Steps

- Learn about [Starting an Agent](./agent.md)
- Explore [Distributed Setup](./distributed.md)
- Configure via [Environment Variables](./environment.md)

# Starting an Agent

## Basic Command

```bash
cdktr start agent
```

Connects to principal at `localhost:5555`

## Options

**--principal-host**: Principal server host
```bash
cdktr start agent --principal-host 192.168.1.100
```

**--principal-port**: Principal server port
```bash
cdktr start agent --principal-port 5570
```

**--max-concurrent**: Max parallel workflows (default: 3)
```bash
cdktr start agent --max-concurrent 10
```

## Common Setups

**Local agent:**
```bash
cdktr start agent
```

**Remote agent:**
```bash
cdktr start agent --principal-host 192.168.1.100
```

**High throughput:**
```bash
cdktr start agent --max-concurrent 10
```

**Full config:**
```bash
cdktr start agent \
  --principal-host 192.168.1.100 \
  --principal-port 5570 \
  --max-concurrent 5
```

## Multiple Agents

Run multiple agents for more capacity:

```bash
# Machine 1
cdktr start agent --max-concurrent 5

# Machine 2
cdktr start agent --principal-host 192.168.1.100 --max-concurrent 5

# Machine 3
cdktr start agent --principal-host 192.168.1.100 --max-concurrent 5
```

Each gets a unique ID and registers independently.

Stop with `Ctrl+C`.
[INFO] Shutting down agent...
[INFO] Waiting for running workflows to complete...
[INFO] Shutdown complete
```

## Multiple Agents

You can run multiple agents on the same machine or across multiple machines:

```bash
# Terminal 1
cdktr start agent --max-concurrent 3

# Terminal 2
cdktr start agent --max-concurrent 3

# Terminal 3 (on another machine)
cdktr start agent --principal-host 192.168.1.100 --max-concurrent 5
```

Each agent gets a unique ID and registers independently.

## Best Practices

1. **Size Appropriately**: Set `--max-concurrent` based on available resources
2. **Monitor Resource Usage**: Watch CPU/memory when tuning concurrency
3. **Network Reliability**: Ensure stable connection to principal
4. **Log Monitoring**: Check agent logs for errors
5. **Restart Strategy**: Use systemd or similar for automatic restart

## Next Steps

- Review [Distributed Setup](./distributed.md) for multi-machine deployments
- Configure [Environment Variables](./environment.md)
- Learn about [Starting a Principal](./principal.md)

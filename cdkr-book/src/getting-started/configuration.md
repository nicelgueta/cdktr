# Configuration

cdktr uses an environment-based configuration system. You can set configuration options via environment variables, a `.env` file, or in some cases command-line arguments.
All configuration options can be listed using:

```bash
cdktr config list
```

## Configuration Options

| Environment Variable | Description | Default Value |
|---------------------|-------------|---------------|
| `CDKTR_LOG_LEVEL` | Default log level | `INFO` |
| `CDKTR_AGENT_MAX_CONCURRENCY` | Maximum number of concurrent workflows an agent can handle | `5` |
| `CDKTR_RETRY_ATTEMPTS` | Number of times to re-attempt a ZMQ request | `20` |
| `CDKTR_DEFAULT_ZMQ_TIMEOUT_MS` | Default timeout for a ZMQ request (milliseconds) | `3000` |
| `CDKTR_PRINCIPAL_HOST` | Hostname of the principal instance | `0.0.0.0` |
| `CDKTR_PRINCIPAL_PORT` | Default port of the principal instance | `5561` |
| `CDKTR_LOGS_LISTENING_PORT` | Listening port for the principal log manager | `5562` |
| `CDKTR_LOGS_PUBLISHING_PORT` | Publishing port for the principal log manager | `5563` |
| `CDKTR_WORKFLOW_DIR` | Default workflow directory | `workflows` |
| `CDKTR_WORKFLOW_DIR_REFRESH_FREQUENCY_S` | Interval to refresh the workflow directory (seconds) | `60` |
| `CDKTR_SCHEDULER_START_POLL_FREQUENCY_MS` | Interval at which the scheduler checks if a workflow is ready to start (milliseconds) | `500` |
| `CDKTR_Q_PERSISTENCE_INTERVAL_MS` | Task queue persistence interval for principal recovery (milliseconds) | `1000` |
| `CDKTR_APP_DATA_DIRECTORY` | App data directory for cdktr instances | `$HOME/.cdktr` |
| `CDKTR_DB_PATH` | Path to the main database for the principal instance | `$HOME/.cdktr/app.db` |
| `CDKTR_TUI_STATUS_REFRESH_INTERVAL_MS` | TUI refresh interval for principal status checks (milliseconds) | `1000` |
| `CDKTR_AGENT_HEARTBEAT_TIMEOUT_MS` | Agent heartbeat timeout - workflows marked as CRASHED if no heartbeat within this duration (milliseconds) | `30000` |
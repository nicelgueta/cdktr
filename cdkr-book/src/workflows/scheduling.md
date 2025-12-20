# Scheduling

cdktr supports automated workflow execution using cron expressions. This allows workflows to run on a regular schedule without manual intervention.

## Cron Expressions

cdktr uses **six-field cron expressions** with seconds precision:

```
┌───────────── second (0-59)
│ ┌───────────── minute (0-59)
│ │ ┌───────────── hour (0-23)
│ │ │ ┌───────────── day of month (1-31)
│ │ │ │ ┌───────────── month (1-12)
│ │ │ │ │ ┌───────────── day of week (0-6) (Sunday=0)
│ │ │ │ │ │
* * * * * *
```

## Basic Examples

### Every Minute
```yaml
cron: "0 * * * * *"
```

### Every 5 Minutes
```yaml
cron: "0 */5 * * * *"
```

### Every Hour at :30
```yaml
cron: "0 30 * * * *"
```

### Daily at 2:00 AM
```yaml
cron: "0 0 2 * * *"
```

### Weekdays at 9:00 AM
```yaml
cron: "0 0 9 * * 1-5"
```

### First Day of Month at Midnight
```yaml
cron: "0 0 0 1 * *"
```

## Special Characters

### Asterisk (*)
Matches any value.

```yaml
cron: "0 * * * * *"  # Every minute
```

### Comma (,)
Specifies multiple values.

```yaml
cron: "0 0 9,17 * * *"  # 9 AM and 5 PM daily
```

### Dash (-)
Specifies a range.

```yaml
cron: "0 0 9 * * 1-5"  # 9 AM on weekdays
```

### Slash (/)
Specifies intervals.

```yaml
cron: "0 */15 * * * *"  # Every 15 minutes
```

## Common Schedules

### High Frequency

```yaml
# Every 30 seconds
cron: "*/30 * * * * *"

# Every 2 minutes
cron: "0 */2 * * * *"

# Every 10 minutes
cron: "0 */10 * * * *"
```

### Hourly

```yaml
# Every hour on the hour
cron: "0 0 * * * *"

# Every 2 hours
cron: "0 0 */2 * * *"

# Every hour at :15
cron: "0 15 * * * *"
```

### Daily

```yaml
# Midnight
cron: "0 0 0 * * *"

# 6 AM
cron: "0 0 6 * * *"

# Noon
cron: "0 0 12 * * *"

# Multiple times per day
cron: "0 0 6,12,18 * * *"  # 6 AM, noon, 6 PM
```

### Weekly

```yaml
# Monday at 9 AM
cron: "0 0 9 * * 1"

# Friday at 5 PM
cron: "0 0 17 * * 5"

# Weekends at 10 AM
cron: "0 0 10 * * 0,6"
```

### Monthly

```yaml
# First of month at midnight
cron: "0 0 0 1 * *"

# Last day of month (use day 28-31 with caution)
# Better to use a script to calculate

# Middle of month
cron: "0 0 0 15 * *"
```

## start_time Field

Use `start_time` to delay the first execution of a scheduled workflow:

```yaml
name: Delayed Workflow
cron: "0 0 * * * *"  # Hourly
start_time: 2025-01-20T12:00:00+00:00
```

The workflow won't run before the specified start time, even if the cron schedule matches.

### Format

ISO 8601 timestamp with timezone:

```
YYYY-MM-DDTHH:MM:SS+00:00
```

### Examples

```yaml
# UTC timezone
start_time: 2025-01-15T09:00:00+00:00

# US Eastern (EST, UTC-5)
start_time: 2025-01-15T09:00:00-05:00

# Europe/London (GMT, UTC+0)
start_time: 2025-01-15T09:00:00+00:00
```

## How Scheduling Works

1. **Workflow Load**: Principal loads workflows from filesystem
2. **Schedule Parse**: Cron expressions parsed and validated
3. **Next Run Calculation**: Next execution time calculated
4. **Priority Queue**: Workflows stored in priority queue by next run time
5. **Scheduler Loop**: Scheduler checks queue and triggers workflows at scheduled times
6. **Re-queue**: After execution, next run time calculated and workflow re-queued

## Manual Triggers

Workflows with schedules can still be triggered manually:

```bash
cdktr task run my-workflow
```

This bypasses the schedule and runs the workflow immediately.

## Workflows Without Schedules

If you omit the `cron` field, the workflow can only be triggered manually:

```yaml
name: On-Demand Workflow
description: Only runs when manually triggered
tasks:
  # ...
```

## Scheduler Behavior

### Multiple Workflows

The scheduler handles multiple workflows efficiently:
- Priority queue ensures timely execution
- Next execution calculated for each workflow independently
- No workflow blocks others

### Clock Skew

If the system time changes (DST, NTP adjustment):
- Scheduler recalculates next run times
- Workflows may run early/late once
- Resumes normal schedule after adjustment

### Missed Schedules

If the principal is down during a scheduled run:
- Missed executions are **not** retroactively triggered
- Next scheduled run proceeds as normal
- Consider using monitoring to detect downtime

## Best Practices

1. **Use Standard Times**: Schedule during off-peak hours (e.g., 2-4 AM)
2. **Avoid Second Precision**: Use minute-level schedules for stability
3. **Consider Dependencies**: Schedule dependent workflows with sufficient gaps
4. **Test Schedules**: Verify cron expressions before deploying
5. **Document Intent**: Add comments explaining schedule reasoning

## Cron Expression Tools

### Online Tools

- [crontab.guru](https://crontab.guru/) - Standard 5-field cron
- [crontab-generator.org](https://crontab-generator.org/) - Interactive generator

Note: These tools use 5-field cron. Add a seconds field (usually `0`) at the start for cdktr.

### Testing

Test your cron expression:

```yaml
# Temporary: run every minute for testing
cron: "0 * * * * *"

# Production: run daily at 2 AM
cron: "0 0 2 * * *"
```

## Examples

### ETL Pipeline (Daily at 3 AM)

```yaml
name: Daily ETL
cron: "0 0 3 * * *"
tasks:
  extract:
    # ...
  transform:
    depends: ["extract"]
    # ...
  load:
    depends: ["transform"]
    # ...
```

### Report Generation (Weekdays at 8 AM)

```yaml
name: Morning Report
cron: "0 0 8 * * 1-5"  # Monday-Friday
tasks:
  generate:
    # ...
  email:
    depends: ["generate"]
    # ...
```

### Monitoring Check (Every 5 Minutes)

```yaml
name: Health Check
cron: "0 */5 * * * *"
tasks:
  check:
    # ...
```

## Next Steps

- Learn about [Task Dependencies](./dependencies.md)
- See [Workflow Examples](./examples.md)
- Explore [Advanced Topics](../advanced.md)

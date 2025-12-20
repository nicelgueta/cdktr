# YAML Structure

## File Location

Place workflows in `./workflows` (configurable via `CDKTR_WORKFLOW_DIR`):

```
workflows/
  backup.yml              → ID: "backup"
  etl/
    daily.yml             → ID: "etl.daily"
```

## Workflow Fields

```yaml
name: Daily Sales Report              # Required: Display name
description: Generate sales reports   # Optional: Description
cron: "0 0 9 * * 1-5"                 # Optional: Schedule (weekdays 9am)
start_time: 2025-01-20T12:00:00+00:00 # Optional: First run time
tasks:                                # Required: Task definitions
  task_id:
    name: Task Name                   # Required
    description: What it does         # Optional
    depends: ["other_task"]           # Optional: Dependencies
    config:                           # Required: Execution config
      !Subprocess
      cmd: python
      args: ["script.py"]
  process:
    name: Process Data
    depends: ["fetch"]
    # ...
```

**Rules:**
- Must contain at least one task
- Task IDs must be unique within the workflow
- Task IDs used in dependency declarations

## Task Structure

```yaml
task-id:
  name: <string> (required)
  description: <string> (optional)
  depends: [<task-id>, ...] (optional)
  config:
    <executable configuration> (required)
```

### Task Fields

#### name (required)

Human-readable task name.

```yaml
name: Download Customer Data
```

#### description (optional)

Description of what the task does.

```yaml
description: Downloads customer data from the API
```

#### depends (optional)

List of task IDs this task depends on.

```yaml
depends: ["fetch_data", "validate_schema"]
```

**Rules:**
- Tasks listed must exist in the workflow
- No circular dependencies allowed
- Empty list `[]` is same as omitting the field

#### config (required)

Executable configuration for the task.

```yaml
config:
  !Subprocess
  cmd: python
  args:
    - script.py
    - --input
    - data.csv
```

See [Task Configuration](./task-config.md) for details.

## Complete Example

```yaml
name: E-commerce Data Pipeline
description: |
  Processes e-commerce transactions daily:
  1. Fetches data from multiple sources
  2. Validates and cleans data
  3. Loads into data warehouse
  4. Sends notification on completion

cron: "0 0 3 * * *"  # Daily at 3 AM
start_time: 2025-01-15T03:00:00+00:00

tasks:
  fetch_orders:
    name: Fetch Orders
    description: Download order data from Shopify API
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/fetch_orders.py

  fetch_customers:
    name: Fetch Customers
    description: Download customer data from CRM
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/fetch_customers.py

  validate:
    name: Validate Data
    description: Check data quality and schema
    depends: ["fetch_orders", "fetch_customers"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/validate.py

  load:
    name: Load to Warehouse
    description: Load validated data to Snowflake
    depends: ["validate"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/load_warehouse.py

  notify:
    name: Send Notification
    description: Notify team of completion
    depends: ["load"]
    config:
      !Subprocess
      cmd: curl
      args:
        - -X
        - POST
        - https://hooks.slack.com/services/XXX
        - -d
        - '{"text": "Pipeline completed"}'
```

## Validation

cdktr validates workflows when they're loaded:

✅ **Valid workflows** are added to the workflow store
❌ **Invalid workflows** are logged and skipped

### Common Validation Errors

1. **Missing required fields**
   ```yaml
   # ERROR: Missing 'name'
   tasks:
     task1:
       config: ...
   ```

2. **Circular dependencies**
   ```yaml
   # ERROR: task_a depends on task_b depends on task_a
   tasks:
     task_a:
       depends: ["task_b"]
     task_b:
       depends: ["task_a"]
   ```

3. **Invalid cron expression**
   ```yaml
   # ERROR: Invalid cron
   cron: "not a cron expression"
   ```

4. **Non-existent dependency**
   ```yaml
   # ERROR: task2 doesn't exist
   tasks:
     task1:
       depends: ["task2"]
   ```

## Best Practices

1. **Use Descriptive Names**: Make workflow and task names self-explanatory
2. **Document with description**: Add context for future maintainers
3. **Organize by Directory**: Group related workflows in subdirectories
4. **Version Control**: Keep workflows in git for change tracking
5. **Test Locally**: Validate workflows before deploying

## Next Steps

- Learn about [Task Configuration](./task-config.md) options
- Explore [Scheduling](./scheduling.md) with cron
- Understand [Task Dependencies](./dependencies.md)
- See real [Workflow Examples](./examples.md)

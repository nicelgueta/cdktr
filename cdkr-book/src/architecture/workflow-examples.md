# Workflow Examples

## Scheduled Task
Cron-based execution.

```yaml
name: Database Backup
cron: "0 0 2 * * *"  # Daily at 2 AM
tasks:
  backup:
    name: Backup DB
    config:
      !Subprocess
      cmd: pg_dump
      args: ["-h", "db.example.com", "-f", "/backups/db.sql"]
```

## Linear Pipeline
Sequential task execution.

```yaml
name: ETL Pipeline
tasks:
  extract:
    name: Extract
    config:
      !Subprocess
      cmd: python
      args: ["extract.py"]

  transform:
    name: Transform
    depends: ["extract"]
    config:
      !Subprocess
      cmd: python
      args: ["transform.py"]

  load:
    name: Load
    depends: ["transform"]
    config:
      !Subprocess
      cmd: python
      args: ["load.py"]
```

## Parallel Tasks
Multiple independent tasks.

```yaml
name: Multi-Source Scraper
tasks:
  scrape_a:
    name: Scrape Source A
    config:
      !Subprocess
      cmd: python
      args: ["scrape_a.py"]

  scrape_b:
    name: Scrape Source B
    config:
      !Subprocess
      cmd: python
      args: ["scrape_b.py"]

  combine:
    name: Combine Results
    depends: ["scrape_a", "scrape_b"]
    config:
      !Subprocess
      cmd: python
      args: ["combine.py"]
```

## UvPython with Dependencies
Managed Python execution with automatic dependency installation.

```yaml
name: Data Analysis
tasks:
  analyze:
    name: Analyze Data
    config:
      !UvPython
      script_path: ./analyze.py
      packages:
        - pandas>=2.3.1,<3.0.0
        - matplotlib>=3.8.0
      working_directory: ./scripts
```

## UvPython Project
For scripts with inline dependencies (PEP 723).

```yaml
name: Report Generator
tasks:
  generate:
    name: Generate Report
    config:
      !UvPython
      script_path: ./generate_report.py
      is_uv_project: true
```

## HTTP Webhook
Trigger external services.

```yaml
name: Notify Slack
tasks:
  notify:
    name: Send Notification
    config:
      !Subprocess
      cmd: curl
      args:
        - -X
        - POST
        - https://hooks.slack.com/services/XXX
        - -d
        - '{"text":"Pipeline complete"}'

  validate_customers:
    name: Validate Customer Data
    depends: ["extract_customers"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/validate.py
        - --input
        - /data/customers.csv

  transform:
    name: Transform and Join
    depends: ["validate_sales", "validate_customers"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/transform.py

  load_warehouse:
    name: Load to Data Warehouse
    depends: ["transform"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/load_warehouse.py

  update_reports:
    name: Update BI Reports
    depends: ["load_warehouse"]
    config:
      !Subprocess
      cmd: curl
      args:
        - -X
        - POST
        - https://bi.example.com/api/refresh
```

## Example 3: Web Scraping

Multi-source web scraping with error handling.

```yaml
name: News Scraper
description: Scrapes news from multiple sources
cron: "0 0 */6 * * *"  # Every 6 hours

tasks:
  scrape_source_1:
    name: Scrape TechCrunch
    config:
      !Subprocess
      cmd: python
      args:
        - scrapers/techcrunch.py

  scrape_source_2:
    name: Scrape Hacker News
    config:
      !Subprocess
      cmd: python
      args:
        - scrapers/hackernews.py

  scrape_source_3:
    name: Scrape Reddit
    config:
      !Subprocess
      cmd: python
      args:
        - scrapers/reddit.py

  deduplicate:
    name: Remove Duplicates
    depends: ["scrape_source_1", "scrape_source_2", "scrape_source_3"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/deduplicate.py

  analyze_sentiment:
    name: Analyze Sentiment
    depends: ["deduplicate"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/sentiment.py

  generate_digest:
    name: Generate Daily Digest
    depends: ["analyze_sentiment"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/generate_digest.py

  send_email:
    name: Send Digest Email
    depends: ["generate_digest"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/send_email.py
```

## Example 4: Machine Learning Pipeline

Model training and deployment workflow.

```yaml
name: ML Model Training
description: Train and deploy ML model
cron: "0 0 1 * * 0"  # Weekly on Sunday at 1 AM

tasks:
  fetch_training_data:
    name: Fetch Training Data
    config:
      !Subprocess
      cmd: python
      args:
        - ml/fetch_data.py
        - --days
        - "7"

  preprocess:
    name: Preprocess Data
    depends: ["fetch_training_data"]
    config:
      !Subprocess
      cmd: python
      args:
        - ml/preprocess.py

  train_model:
    name: Train Model
    depends: ["preprocess"]
    config:
      !Subprocess
      cmd: python
      args:
        - ml/train.py
        - --epochs
        - "100"

  evaluate:
    name: Evaluate Model
    depends: ["train_model"]
    config:
      !Subprocess
      cmd: python
      args:
        - ml/evaluate.py

  deploy:
    name: Deploy to Production
    depends: ["evaluate"]
    config:
      !Subprocess
      cmd: python
      args:
        - ml/deploy.py
        - --environment
        - production
```

## Example 5: Monitoring and Alerting

System health check workflow.

```yaml
name: Health Check
description: Check system health and alert on issues
cron: "0 */5 * * * *"  # Every 5 minutes

tasks:
  check_api:
    name: Check API Health
    config:
      !Subprocess
      cmd: curl
      args:
        - -f
        - https://api.example.com/health

  check_database:
    name: Check Database Connection
    config:
      !Subprocess
      cmd: psql
      args:
        - -h
        - db.example.com
        - -U
        - monitor
        - -c
        - SELECT 1;

  check_disk_space:
    name: Check Disk Space
    config:
      !Subprocess
      cmd: bash
      args:
        - -c
        - df -h / | awk 'NR==2 {if ($5+0 > 80) exit 1}'

  alert:
    name: Send Alert if Any Failed
    depends: ["check_api", "check_database", "check_disk_space"]
    config:
      !Subprocess
      cmd: python
      args:
        - scripts/send_alert.py
```

## Example 6: Report Generation

Generate and distribute reports.

```yaml
name: Weekly Sales Report
description: Generate weekly sales report
cron: "0 0 9 * * 1"  # Monday at 9 AM

tasks:
  query_data:
    name: Query Sales Data
    config:
      !Subprocess
      cmd: python
      args:
        - reports/query_sales.py
        - --period
        - last_week

  generate_charts:
    name: Generate Charts
    depends: ["query_data"]
    config:
      !Subprocess
      cmd: python
      args:
        - reports/generate_charts.py

  create_pdf:
    name: Create PDF Report
    depends: ["generate_charts"]
    config:
      !Subprocess
      cmd: python
      args:
        - reports/create_pdf.py

  email_report:
    name: Email Report to Team
    depends: ["create_pdf"]
    config:
      !Subprocess
      cmd: python
      args:
        - reports/email_report.py
        - --recipients
        - team@example.com
```

## Next Steps

- Review [YAML Structure](./yaml-structure.md) for details
- Learn about [Task Configuration](./task-config.md)
- Explore [Scheduling](./scheduling.md) options
- Understand [Task Dependencies](./dependencies.md)

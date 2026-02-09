# Event Listeners & Scheduler

cdktr is fundamentally an **event-driven system**. Unlike traditional workflow engines that might poll databases or rely on fixed schedules alone, cdktr treats everything—including scheduled executions—as events. Workflows are triggered by events, whether those events are time-based (cron schedules), user-initiated (CLI commands), or custom external triggers (webhooks, file changes, message queues, etc.).

This event-driven architecture provides tremendous flexibility. The same workflow can be triggered by a cron schedule during regular business hours and also triggered on-demand when a webhook receives a deployment notification. There's no special configuration needed—workflows simply respond to whatever events you send their way.

## The Event-Driven Philosophy

In cdktr, an **event** is anything that says "run this workflow now." Events flow into the principal via its ZeroMQ API, and the principal's response is always the same: add the requested workflow to the task queue. The event source doesn't need to know or care about agents, task execution, or DAGs—it simply says "please run workflow X" and the system handles the rest.

This decoupling is powerful. Your event sources can be simple Python scripts, complex monitoring systems, CI/CD pipelines, or even other workflow engines. As long as they can send a ZeroMQ message, they can trigger cdktr workflows.

## The Scheduler: An Event Listener Implementation

The integrated scheduler is cdktr's canonical example of an event listener. It monitors cron expressions defined in workflow YAML files and triggers those workflows at the appointed times. But here's the key insight: **the scheduler is itself just an implementation of cdktr's event listener interface**.

The scheduler doesn't get special treatment from the principal. It communicates through the same ZeroMQ API that external event listeners use. When a workflow's scheduled time arrives, the scheduler sends a workflow execution request to the principal, just like any other event source would.

### How the Scheduler Works

On startup, the scheduler:

1. Queries the principal for all workflows via the ZeroMQ API
2. Filters to only those with valid cron expressions
3. Calculates the next execution time for each scheduled workflow
4. Builds a priority queue ordered by next execution time

The scheduler then enters its event loop:

1. Checks if the next workflow in the priority queue is ready to run (current time >= scheduled time)
2. If not ready, sleeps for 500 milliseconds and checks again
3. When a workflow is ready, sends a workflow execution request to the principal
4. Calculates when that workflow should run next and re-adds it to the priority queue
5. Repeats indefinitely

The scheduler runs a background refresh loop that queries the principal every 60 seconds for workflow definitions. If new workflows appear or existing ones change, the scheduler updates its internal priority queue accordingly. This means you can deploy new scheduled workflows without restarting the principal—they'll be picked up automatically within a minute.

### Graceful Degradation

If no workflows have cron schedules defined, the scheduler simply doesn't start. The principal continues operating normally, handling manual workflow triggers and external events. The scheduler is truly optional.

## Creating Custom Event Listeners

The real power of cdktr's event-driven architecture emerges when you create custom event listeners tailored to your specific needs. cdktr provides two primary ways to build event listeners: native implementations in Rust, and external implementations in Python (or any language that can speak ZeroMQ).

### Event Listeners in Rust

For high-performance, tightly-integrated event listeners, you can implement the `EventListener` trait in Rust. This trait defines a simple contract:

```rust
#[async_trait]
pub trait EventListener<T> {
    async fn start_listening(&mut self) -> Result<(), GenericError>;
    async fn run_workflow(&mut self, workflow_id: &str) -> Result<(), GenericError>;
}
```

The `start_listening()` method is where your event detection logic lives. It typically runs in an infinite loop, waiting for events to occur. When an event happens that should trigger a workflow, you call `run_workflow()` with the workflow ID, and the trait provides a default implementation that sends the execution request to the principal via ZeroMQ.

**Example: File Watcher Event Listener**

Here's how you might implement a file watcher that triggers workflows when files change:

```rust
use async_trait::async_trait;
use cdktr_events::EventListener;
use notify::{Watcher, RecursiveMode, Event};
use std::sync::mpsc;

pub struct FileWatcherListener {
    watch_path: String,
    workflow_id: String,
}

#[async_trait]
impl EventListener<Event> for FileWatcherListener {
    async fn start_listening(&mut self) -> Result<(), GenericError> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(Path::new(&self.watch_path), RecursiveMode::Recursive)?;

        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    if event.kind.is_modify() {
                        info!("File modified: {:?}, triggering workflow", event.paths);
                        self.run_workflow(&self.workflow_id).await?;
                    }
                }
                Ok(Err(e)) => error!("Watch error: {:?}", e),
                Err(e) => error!("Channel error: {:?}", e),
            }
        }
    }
}
```

This listener watches a directory for file changes and triggers a workflow whenever a modification occurs. The `run_workflow()` call handles all the ZeroMQ communication with the principal.

### Triggering Workflows from Python (cdktr-py)

For teams more comfortable with Python, or for rapid prototyping, cdktr provides the `cdktr-py` library. This library offers a Python interface to cdktr's ZeroMQ API, making it simple to trigger workflows from Python applications.

**Example: Webhook Server**

Here's a Python application that triggers workflows in response to HTTP webhooks:

```python
from cdktr import Principal
from flask import Flask, request

app = Flask(__name__)
principal = Principal(host="localhost", port=5561)

@app.route('/trigger/<workflow_id>', methods=['POST'])
def trigger_workflow(workflow_id):
    payload = request.get_json()
    result = principal.run_workflow(workflow_id)
    if result.success:
        return {"status": "triggered", "workflow": workflow_id}, 200
    else:
        return {"status": "failed", "error": result.error}, 500

if __name__ == "__main__":
    app.run(host='0.0.0.0', port=8080)
```

This creates an HTTP endpoint at `/trigger/<workflow_id>` that accepts POST requests. When a request arrives, it triggers the specified workflow via the cdktr principal.

**Example: Message Queue Consumer**

Here's an application that consumes messages from RabbitMQ and triggers workflows:

```python
from cdktr import Principal
import pika
import json

principal = Principal(host="localhost", port=5561)

def main():
    connection = pika.BlockingConnection(
        pika.ConnectionParameters(host='localhost')
    )
    channel = connection.channel()
    channel.queue_declare(queue='cdktr-workflows', durable=True)

    def callback(ch, method, properties, body):
        try:
            message = json.loads(body)
            workflow_id = message.get('workflow_id')

            if workflow_id:
                result = principal.run_workflow(workflow_id)
                if result.success:
                    ch.basic_ack(delivery_tag=method.delivery_tag)
                else:
                    ch.basic_nack(delivery_tag=method.delivery_tag, requeue=True)
        except Exception as e:
            print(f"Error processing message: {e}")
            ch.basic_nack(delivery_tag=method.delivery_tag, requeue=True)

    channel.basic_qos(prefetch_count=1)
    channel.basic_consume(queue='cdktr-workflows', on_message_callback=callback)

    print("Listening for workflow triggers...")
    channel.start_consuming()

if __name__ == "__main__":
    main()
```

**Example: Database Change Monitor**

Monitor a PostgreSQL database for changes and trigger workflows:

```python
from cdktr import Principal
import psycopg2
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT
import select

principal = Principal(host="localhost", port=5561)

def main():
    conn = psycopg2.connect("postgresql://localhost/mydb")
    conn.set_isolation_level(ISOLATION_LEVEL_AUTOCOMMIT)

    cursor = conn.cursor()
    cursor.execute("LISTEN workflow_triggers;")

    print("Listening for database notifications...")

    while True:
        if select.select([conn], [], [], 5) == ([], [], []):
            continue
        else:
            conn.poll()
            while conn.notifies:
                notify = conn.notifies.pop(0)
                workflow_id = notify.payload
                print(f"Received notification: {workflow_id}")
                principal.run_workflow(workflow_id)

if __name__ == "__main__":
    main()
```

### The Principal API

The `cdktr-py` library provides a `Principal` class that handles ZeroMQ communication:

```python
from cdktr import Principal

principal = Principal(host="localhost", port=5561)

# Trigger a workflow
result = principal.run_workflow("my-workflow")
if result.success:
    print("Workflow triggered successfully!")
else:
    print(f"Failed: {result.error}")

# Other API methods
principal.ping()
principal.list_workflows()
principal.query_logs(workflow_id="my-workflow", limit=10)
principal.get_recent_workflow_statuses()
principal.get_registered_agents()
```

## Real-World Workflow Triggering Patterns

The Principal API enables powerful workflow orchestration patterns:

**CI/CD Integration**: Deploy code, send a webhook to trigger a cdktr workflow that runs tests, migrations, and health checks.

**Data Pipeline Triggers**: When new data lands in S3, trigger a workflow that processes, validates, and loads it into your warehouse.

**Monitoring and Alerting**: When your monitoring system detects an anomaly, trigger a remediation workflow that attempts automatic fixes and notifies the team.

**User Actions**: When a user performs a specific action in your application, trigger a workflow that sends emails, updates analytics, and logs to your data warehouse.

**Cross-System Orchestration**: Use event listeners to bridge different systems—when System A completes a task, trigger a cdktr workflow that kicks off related work in System B.

## Configuration and Deployment

Event listeners are separate processes from the principal and agents. You deploy them alongside your cdktr infrastructure:

**Development**: Run event listeners in the same terminal or IDE where you're running the principal, useful for testing and debugging.

**Production**: Deploy event listeners as systemd services, Docker containers, or Kubernetes pods, ensuring they have network connectivity to the principal's ZeroMQ port.

Event listeners should be treated as first-class components of your workflow infrastructure, with proper monitoring, logging, and error handling.

## The Power of Events

By treating everything as events, cdktr provides a unified model for workflow triggering. Whether a workflow runs at 3 AM every day via the scheduler, gets triggered by a deployment webhook, or responds to a file change, the execution path is identical. This consistency makes cdktr predictable and easy to reason about, while the extensibility of the event listener interface ensures you're never locked into a predefined set of trigger types.

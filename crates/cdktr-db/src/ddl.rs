pub static DDL: [&'static str; 5] = [
    // TYPES

    // should match rust enum RunStatus
    "create type if not exists RunStatus
    as ENUM (
        'PENDING',
        'RUNNING',
        'WAITING',
        'COMPLETED',
        'FAILED',
        'CRASHED'
    )
    ",
    // type of run
    "create type if not exists RunType
    as ENUM (
        'Workflow',
        'Task'
    )
    ",
    // Main DDL
    // Create the logstore table
    "create table IF NOT EXISTS logstore
    (
        workflow_id TEXT,
        workflow_name TEXT,
        workflow_instance_id TEXT,
        task_name TEXT,
        task_instance_id TEXT,
        timestamp_ms BIGINT,
        level TEXT,
        payload TEXT,
    );",
    // Create the workflow run status table - insert only
    "create table IF NOT EXISTS workflow_run_status
    (
        workflow_id TEXT,
        workflow_instance_id TEXT,
        status RunStatus,
        timestamp_ms BIGINT,
    );",
    // Create the task run status table - insert only
    "create table IF NOT EXISTS task_run_status
    (
        task_id TEXT,
        task_instance_id TEXT,
        workflow_instance_id TEXT,
        status RunStatus,
        timestamp_ms BIGINT,
    );",
];

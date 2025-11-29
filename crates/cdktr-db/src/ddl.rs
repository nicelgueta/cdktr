pub static DDL: [&'static str; 4] = [
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
    // Create the runinfo - this is an insert only table
    "create table IF NOT EXISTS run_status
    (
        object_id TEXT,
        object_instance_id TEXT,
        run_type RunType,
        status RunStatus,
        timestamp_ms BIGINT,
    );",
];

pub static DDL: [&'static str; 1] = [
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
];

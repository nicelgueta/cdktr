// @generated automatically by Diesel CLI.

diesel::table! {
    schedules (id) {
        id -> Integer,
        task_name -> Text,
        command -> Text,
        args -> Text,
        cron -> Text,
        timestamp_created -> Integer,
        next_run_timestamp -> Integer,
    }
}

// @generated automatically by Diesel CLI.

diesel::table! {
    schedules (id) {
        id -> Integer,
        task_name -> Text,
        task_type -> Text,
        command -> Text,
        args -> Nullable<Text>,
        cron -> Nullable<Text>,
        timestamp_created -> Integer,
        next_run_timestamp -> Integer,
    }
}

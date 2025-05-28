// @generated automatically by Diesel CLI.

diesel::table! {
    tasks (id) {
        id -> Integer,
        task_name -> Text,
        task_type -> Text,
        command -> Text,
        args -> Nullable<Text>,
        cron -> Nullable<Text>,
        timestamp_created -> BigInt,
        next_run_timestamp -> BigInt,
    }
}

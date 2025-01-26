-- Your SQL goes here
CREATE TABLE IF NOT EXISTS `schedules`(
	`id` INTEGER PRIMARY KEY NOT NULL,
	`task_name` TEXT NOT NULL,
	`task_type` TEXT NOT NULL,
	`command` TEXT NOT NULL,
	`args` TEXT NULL,
	`cron` TEXT NULL,
	`timestamp_created` INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
	`next_run_timestamp` INTEGER NOT NULL
);

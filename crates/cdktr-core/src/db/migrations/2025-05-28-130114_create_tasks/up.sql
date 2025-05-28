-- Your SQL goes here
CREATE TABLE IF NOT EXISTS `tasks`(
	`id` INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
	`task_name` TEXT NOT NULL,
	`task_type` TEXT NOT NULL,
	`command` TEXT NOT NULL,
	`args` TEXT NULL,
	`cron` TEXT NULL,
	`timestamp_created` BIGINT NOT NULL DEFAULT (strftime('%s', 'now')),
	`next_run_timestamp` BIGINT NOT NULL
);

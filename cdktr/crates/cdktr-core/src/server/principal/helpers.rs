/// API module to provide all of the principal message handling
/// utilities
///
use crate::{
    db::models::{NewScheduledTask, ScheduledTask},
    models::{Task, TaskStatus},
    server::models::ClientResponseMessage,
    utils::data_structures::{AgentPriorityQueue, AsyncQueue},
};
use diesel::prelude::*;
use diesel::RunQueryDsl;
use log::{debug, info, trace};

pub fn handle_create_task(
    db_cnxn: &mut SqliteConnection,
    scheduled_task: NewScheduledTask,
) -> (ClientResponseMessage, usize) {
    use crate::db::schema::schedules;
    let result = diesel::insert_into(schedules::table)
        .values(&scheduled_task)
        .execute(db_cnxn);
    match result {
        Ok(_v) => (ClientResponseMessage::Success, 0),
        Err(e) => (ClientResponseMessage::ServerError(e.to_string()), 0),
    }
}

pub fn handle_list_tasks(db_cnxn: &mut SqliteConnection) -> (ClientResponseMessage, usize) {
    use crate::db::schema::schedules::dsl::*;
    let results: Result<Vec<ScheduledTask>, diesel::result::Error> =
        schedules.select(ScheduledTask::as_select()).load(db_cnxn);
    match results {
        Ok(res) => match serde_json::to_string(&res) {
            Ok(json_str) => (ClientResponseMessage::SuccessWithPayload(json_str), 0),
            Err(e) => (
                ClientResponseMessage::ServerError(format!(
                    "Failed to convert data to JSON string. Got error: {}",
                    e.to_string()
                )),
                0,
            ),
        },
        Err(e) => (
            ClientResponseMessage::ServerError(format!(
                "Failed to query data from database. Got error: {}",
                e.to_string()
            )),
            0,
        ),
    }
}

pub fn handle_delete_task(
    db_cnxn: &mut SqliteConnection,
    task_id: i32,
) -> (ClientResponseMessage, usize) {
    use crate::db::schema::schedules::dsl::*;
    let result = diesel::delete(schedules.filter(id.eq(task_id))).execute(db_cnxn);
    match result {
        Ok(num_affected) => {
            if num_affected >= 1 {
                (ClientResponseMessage::Success, 0)
            } else {
                (
                    ClientResponseMessage::Unprocessable(format!(
                        "No records found for task_id {task_id}"
                    )),
                    0,
                )
            }
        }
        Err(e) => {
            let msg = format!(
                "Failed to convert data to JSON string. Got error: {}",
                e.to_string()
            );
            (ClientResponseMessage::ServerError(msg), 0)
        }
    }
}

pub async fn handle_agent_task_status_update(
    live_agents: AgentPriorityQueue,
    task_id: &str,
    status: &TaskStatus,
) -> (ClientResponseMessage, usize) {
    // TODO: do something with the task id.
    //
    // TODO
    (
        ClientResponseMessage::SuccessWithPayload("TBD".to_string()),
        0,
    )
}

pub async fn handle_add_task(
    task: Task,
    queue: &mut AsyncQueue<Task>,
) -> (ClientResponseMessage, usize) {
    info!(
        "Adding task to global task queue - task -> {}",
        task.to_string()
    );
    queue.put(task).await;
    info!("Current task queue size: {}", queue.size().await);
    (ClientResponseMessage::Success, 0)
}

pub async fn handle_fetch_task(
    task_queue: &mut AsyncQueue<Task>,
    agent_id: String,
) -> (ClientResponseMessage, usize) {
    // TODO: do something with the agent ID like this agent is allowed to
    // process this type of task
    let task_res = task_queue.get().await;
    if let Some(task) = task_res {
        info!(
            "Agent {agent_id} requested task | Sending task -> {}",
            task.to_string()
        );
        info!("Current task queue size: {}", task_queue.size().await);
        (
            ClientResponseMessage::SuccessWithPayload(task.to_string()),
            0,
        )
    } else {
        trace!("No task found - sending empty success to client");
        (ClientResponseMessage::Success, 0)
    }
}

#[cfg(test)]
mod tests {

    use crate::{models::Task, utils::data_structures::AsyncQueue};

    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

    fn setup_db() -> SqliteConnection {
        let mut cnxn = SqliteConnection::establish(":memory:").unwrap();
        cnxn.run_pending_migrations(MIGRATIONS).unwrap();
        cnxn
    }

    #[test]
    fn test_handle_create_task_happy() {
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744,
        };
        assert_eq!(
            handle_create_task(&mut db_cnxn, task),
            (ClientResponseMessage::Success, 0)
        )
    }

    #[test]
    fn test_handle_list_tasks_empty_db() {
        let mut db_cnxn = setup_db();
        assert_eq!(
            handle_list_tasks(&mut db_cnxn),
            (
                ClientResponseMessage::SuccessWithPayload("[]".to_string()),
                0
            )
        )
    }

    #[test]
    fn test_handle_list_tasks_1_in_db() {
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744,
        };
        handle_create_task(&mut db_cnxn, task);

        let (resp, exit_code) = handle_list_tasks(&mut db_cnxn);
        assert_eq!(exit_code, 0);
        match resp {
            ClientResponseMessage::SuccessWithPayload(json_str) => {
                let tasks: Vec<ScheduledTask> = serde_json::from_str(&json_str).unwrap();
                assert_eq!(tasks.len(), 1);
            }
            _ => panic!("Expected SuccessWithPayload but got {:?}", resp),
        }
    }

    #[test]
    fn test_handle_delete_task_happy() {
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744,
        };
        handle_create_task(&mut db_cnxn, task);
        assert_eq!(
            handle_delete_task(&mut db_cnxn, 1),
            (ClientResponseMessage::Success, 0)
        )
    }

    #[test]
    fn test_handle_delete_task_not_found() {
        let mut db_cnxn = setup_db();
        let task = NewScheduledTask {
            task_name: "echo hello".to_string(),
            task_type: "PROCESS".to_string(),
            command: "echo".to_string(),
            args: "hello".to_string(),
            cron: "0 3 * * * *".to_string(),
            next_run_timestamp: 1720313744,
        };
        handle_create_task(&mut db_cnxn, task);
        assert_eq!(
            handle_delete_task(&mut db_cnxn, 2),
            (
                ClientResponseMessage::Unprocessable("No records found for task_id 2".to_string()),
                0
            )
        )
    }

    #[tokio::test]
    async fn test_add_task_to_queue() {
        let mut queue = AsyncQueue::new();
        let task = Task::try_from("PROCESS|echo|hello world".to_string()).unwrap();
        handle_add_task(task, &mut queue).await;
        assert_eq!(queue.size().await, 1)
    }

    #[tokio::test]
    async fn test_fetch_task_no_tasks() {
        let mut task_queue = AsyncQueue::new();
        assert_eq!(task_queue.size().await, 0);

        let (cli_msg, code) = handle_fetch_task(&mut task_queue, "1234".to_string()).await;

        assert_eq!(task_queue.size().await, 0);
        assert_eq!(cli_msg, ClientResponseMessage::Success);
        assert_eq!(code, 0);
    }

    #[tokio::test]
    async fn test_fetch_task_2_tasks() {
        let mut task_queue = AsyncQueue::new();

        // put some dummy tasks onthe queue
        task_queue
            .put(Task::try_from("PROCESS|echo|hello world".to_string()).unwrap())
            .await;
        task_queue
            .put(Task::try_from("PROCESS|echo|hello world".to_string()).unwrap())
            .await;

        assert_eq!(task_queue.size().await, 2);

        let (cli_msg, code) = handle_fetch_task(&mut task_queue, "1234".to_string()).await;

        assert_eq!(task_queue.size().await, 1);

        assert_eq!(
            cli_msg,
            ClientResponseMessage::SuccessWithPayload("PROCESS|echo|hello world".to_string())
        );
        assert_eq!(code, 0);

        assert_eq!(cli_msg.payload(), "PROCESS|echo|hello world".to_string())
    }
}

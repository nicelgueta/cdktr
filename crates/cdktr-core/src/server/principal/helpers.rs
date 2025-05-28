/// API module to provide all of the principal message handling
/// utilities
///
use crate::{
    db::models::{NewScheduledTask, ScheduledTask},
    models::{traits::ToTask, Task, TaskStatus},
    server::models::ClientResponseMessage,
    utils::data_structures::{AgentPriorityQueue, AsyncQueue},
};
use diesel::prelude::*;
use diesel::RunQueryDsl;
use log::{info, trace};

pub fn handle_list_tasks(db_cnxn: &mut SqliteConnection) -> (ClientResponseMessage, usize) {
    use crate::db::schema::tasks::dsl::*;
    let results: Result<Vec<ScheduledTask>, diesel::result::Error> =
        tasks.select(ScheduledTask::as_select()).load(db_cnxn);
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

pub async fn handle_run_task(
    task_id: i32,
    db_cnxn: &mut SqliteConnection,
    queue: &mut AsyncQueue<Task>,
) -> (ClientResponseMessage, usize) {
    use crate::db::schema::tasks::dsl::*;
    info!("Staging task -> {}", task_id.to_string());
    let task_res: Result<ScheduledTask, diesel::result::Error> = tasks
        .filter(id.eq(task_id))
        .select(ScheduledTask::as_select())
        .first(db_cnxn);
    let task = if let Ok(task) = task_res {
        task.to_task()
    } else {
        return (
            ClientResponseMessage::ServerError(format!(
                "Failed to retreive task with id {}",
                task_id
            )),
            0,
        );
    };
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
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/db/migrations");

    fn setup_db() -> SqliteConnection {
        let mut cnxn = SqliteConnection::establish(":memory:").unwrap();
        cnxn.run_pending_migrations(MIGRATIONS).unwrap();
        cnxn
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
        // TODO
    }

    #[tokio::test]
    async fn test_handle_run_task() {
        // TODO
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

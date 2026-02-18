use cdktr_core::exceptions::GenericError;
use log::info;

mod scheduler;

/// Spawns the Scheduler in a separate coroutine
pub async fn start_scheduler() -> Result<(), GenericError> {
    let mut scheduler = scheduler::Scheduler::new(None).await?;
    info!("Starting scheduler");
    scheduler.spawn_refresh_loop().await;
    scheduler.start_listening().await?;
    Ok(())
}

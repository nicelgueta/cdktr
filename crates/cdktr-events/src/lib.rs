use cdktr_core::exceptions::GenericError;
use log::info;

use crate::traits::EventListener;

mod scheduler;
mod traits;

/// Spawns the Scheduler in a separate coroutine
pub async fn start_scheduler() -> Result<(), GenericError> {
    let mut scheduler = scheduler::Scheduler::new().await?;
    info!("Starting scheduler");
    scheduler.spawn_refresh_loop().await;
    scheduler.start_listening().await?;
    Ok(())
}

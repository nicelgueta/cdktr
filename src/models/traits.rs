use crate::utils::AsyncQueue;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use super::FlowExecutionResult;

/// An Executor is a trait that defines the interface for components that
/// are responsible for executing tasks. The executor is responsible for
/// running the task and sending the result back to the caller
#[async_trait]
pub trait Executor {
    fn new(command: &str, args: Option<Vec<String>>) -> Self;
    async fn run(&self, tx: Sender<String>) -> FlowExecutionResult;
}

/// The event listener trait is for implementing components that
/// listen to external events and place onto a Queue. T refers to
/// the item that will be placed on the queue upon each event.
#[async_trait]
pub trait EventListener<T> {
    async fn start_listening_loop(&self, out_queue: AsyncQueue<T>);
}
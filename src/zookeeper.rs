
use std::{path::Component, thread, time};
use std::sync::{Arc, Mutex};
use crate::{
    executor::{Executor, FlowExecutionResult},
    event::run_zmq_listener
};

#[derive(Debug)]
pub struct Zookeeper {
    max_threads: usize,
    thread_counter: Arc<Mutex<usize>>
}

#[derive(Debug, PartialEq)]
pub enum ZookeeperError {
    SpawnError(String),
    FlowError(String),
    Other
}

impl Zookeeper {
    pub fn new(max_threads: usize) -> Self {
        Self {
            max_threads, thread_counter: Arc::new(Mutex::new(0))
        }
    }
    /// Run a command in a spawned thread.
    /// Note that there is no join handle to wait on thread completion
    pub fn run_in_executor(&mut self, cmd: String, args: Vec<String>) -> Result<(),ZookeeperError> {
        let counter_ptr = self.thread_counter.clone();
        if *counter_ptr.lock().unwrap() >= self.max_threads {
            return Err(ZookeeperError::SpawnError("Cannot spawn new process - max_threads reached".to_string()))
        };
        thread::spawn(move ||{
            // inform the zookeeper of another running process
            let mut counter = counter_ptr.lock().unwrap();
            *counter+=1;
            drop(counter); // release the lock

            let executor = Executor::new(&cmd, Some(args));
            let _flow_result = executor.run(|x|println!("stdout from executor: {}", x));
            // TODO: handle the result

            // inform zookeeper process has terminated
            let mut counter = counter_ptr.lock().unwrap();
            *counter-=1;
            drop(counter);
        });
        Ok(())
    }

    /// blocks main process to wait on all running threads to complete
    pub fn wait_on_threads(&self) {
        let thread_counter = self.thread_counter.clone();
        loop {
            if *thread_counter.lock().unwrap() == 0 {
                break
            }
        }
    }
    /// blocks main thread to run an event loop bound to a zmq
    /// feed. TODO: outline the basic loop here but switch
    /// to using dependency injection so as to swtich the type
    /// of event handler this loop will use
    pub fn main_event_loop(&mut self, host: &str, port: usize) {
        let callback = |msg: String| {
            let splits: Vec<&str> = msg.split("|").collect();
            let cmd = splits[0];
            let arg = splits[1];
            self.run_in_executor(cmd.to_string(), vec![
                arg.to_string()
            ]).unwrap();
        };
        run_zmq_listener(host, port, callback);
    }   


}

#[cfg(test)]
mod tests {
    use core::time;
    use std::thread;

    use crate::zookeeper::ZookeeperError;

    use super::Zookeeper;

    #[test]
    fn test_run_single_flow() {
        let mut zk = Zookeeper::new(1);
        let result = zk.run_in_executor("echo".to_string(), vec!["Running test_run_flow".to_string()]);
        assert_eq!(result.unwrap(), ());
    }

    #[test]
    fn test_run_single_flow_slow() {
        let mut zk = Zookeeper::new(1);
        let result = zk.run_in_executor("python".to_string(), vec!["s.py".to_string(), "3".to_string()]);
        assert_eq!(result.unwrap(), ());
        let second = time::Duration::from_secs(1);
        thread::sleep(second);
        zk.wait_on_threads()
        
    }

    #[test]
    fn test_run_multiple_flow_slow() {
        let mut zk = Zookeeper::new(3);
        let result1 = zk.run_in_executor("python".to_string(), vec!["s.py".to_string(), "3".to_string()]);
        let result2 = zk.run_in_executor("python".to_string(), vec!["s.py".to_string(), "4".to_string()]);
        let result3 = zk.run_in_executor("python".to_string(), vec!["s.py".to_string(), "2".to_string()]);
        assert_eq!(result1.unwrap(), ());
        assert_eq!(result2.unwrap(), ());
        assert_eq!(result3.unwrap(), ());
        let second = time::Duration::from_secs(1);
        thread::sleep(second);
        zk.wait_on_threads()
        
    }

    #[test]
    fn test_run_multiple_flow_too_many_threads() {
        let mut zk = Zookeeper::new(2);
        let result1 = zk.run_in_executor("python".to_string(), vec!["s.py".to_string(), "3".to_string()]);
        let result2 = zk.run_in_executor("python".to_string(), vec!["s.py".to_string(), "4".to_string()]);
        assert_eq!(result1.unwrap(), ());
        assert_eq!(result2.unwrap(), ());

        let second = time::Duration::from_secs(1);
        thread::sleep(second);

        let result3 = zk.run_in_executor("python".to_string(), vec!["s.py".to_string(), "2".to_string()]);
        match result3 {
            Ok(()) => panic!("Adding another thread beyond max threads should error"),
            Err(e) => assert_eq!(e, ZookeeperError::SpawnError("Cannot spawn new process - max_threads reached".to_string()))
        }
        zk.wait_on_threads()
        
    }
}
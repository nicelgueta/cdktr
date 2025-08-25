use cdktr_core::exceptions::GenericError;
use duckdb::{arrow::array::RecordBatch, Connection, Params};
use log::warn;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};

mod ddl;

pub trait DBRecordBatch<T>
where
    Self: Sized,
{
    fn to_record_batch(&self) -> Result<RecordBatch, GenericError>;
    fn from_record_batch(rb: RecordBatch) -> Result<Self, GenericError>;
}

/// Thread-safe db client mutex to be shared across a cdktr principal
/// application instance. All processes and coroutines should use
/// the same connection to follow duckdb's single process read/write rule.
#[derive(Clone)]
pub struct DBClient {
    cnxn: Arc<Mutex<Connection>>,
}

impl DBClient {
    pub fn new(app_db_path: Option<&str>) -> Result<Self, GenericError> {
        let inner_cnxn = if let Some(path) = app_db_path {
            Connection::open(path).expect(&format!(
                "No connectable database can be found at: {}",
                path
            ))
        } else {
            Connection::open_in_memory().expect("Failed to open in-memory duckdb connection")
        };
        // idempotently run any new ddl
        gen_ddl(&inner_cnxn)?;
        Ok(Self {
            cnxn: Arc::new(Mutex::new(inner_cnxn)),
        })
    }

    pub async fn execute<P: Params>(&self, q: &str, params: P) -> Result<usize, GenericError> {
        let lock = self.cnxn.lock().await;
        lock.execute(q, params)
            .map_err(|e| GenericError::DBError(e.to_string()))
    }

    // Loads a batch of records into the database. Returns the input batch as the Err variant for additional
    // error processing outside of the function
    pub async fn batch_load<T, V: DBRecordBatch<T> + Clone>(
        &self,
        table_name: &str,
        batch: V,
    ) -> Result<(), V> {
        let lock = self.cnxn.lock().await;
        let mut app: duckdb::Appender<'_> = lock
            .appender(table_name)
            .expect("Unable to create appender to db - todo: handle");
        let rb = match batch.to_record_batch() {
            Ok(bt) => bt,
            Err(e) => {
                warn!(
                    "Could not create record batch from records - aborting insert. Orig error: {}",
                    e.to_string()
                );
                return Err(batch);
            }
        };
        match app.append_record_batch(rb) {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!(
                    "Failed to insert record batch into database. Orig error: {}",
                    e.to_string()
                );
                Err(batch)
            }
        }
    }

    pub async fn lock_inner_client(&self) -> MutexGuard<Connection> {
        self.cnxn.lock().await
    }
}

fn gen_ddl<'a>(cnxn: &'a Connection) -> Result<(), GenericError> {
    for ddl_statement in ddl::DDL {
        cnxn.execute(ddl_statement, [])
            .map_err(|e| GenericError::DBError(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckdb::params;
    #[tokio::test]
    async fn test_get_db_client_in_memory() {
        let cli = DBClient::new(None).unwrap();
        assert!(cli.execute("select 1", params![]).await.is_ok());
    }
}

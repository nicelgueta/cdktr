use cdktr_core::exceptions::GenericError;
use duckdb::{Connection, Params, arrow};
use log::warn;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

mod ddl;

pub trait DBRecordBatch<T> {
    fn from_record_batch(batch: arrow::array::RecordBatch) -> Result<Vec<T>, GenericError>;
    fn to_record_batch(&self) -> Result<arrow::array::RecordBatch, GenericError>;
}

/// Proc macro to conveniently provide the implementation for converting
/// a given struct to a record batch for easy loading into the database
#[macro_export]
macro_rules! impl_dbrecordbatch {
    (
        $struct:ident, $vec:ty, {
            $($field:ident => $arrow_ty:ident),* $(,)?
        }
    ) => {
        macro_rules! builder_path {
            (UInt64) => { ::duckdb::arrow::array::UInt64Builder };
            (Utf8) => { ::duckdb::arrow::array::StringBuilder };
            // add more types
        }

        macro_rules! array_builder {
            (UInt64) => { ::duckdb::arrow::array::UInt64Array };
            (Utf8) => { ::duckdb::arrow::array::StringArray };
            // add more types
        }
        impl ::cdktr_db::DBRecordBatch<$struct> for $vec {
            fn from_record_batch(batch: ::duckdb::arrow::array::RecordBatch) -> Result<$vec, ::cdktr_core::exceptions::GenericError> {
                $(
                    let $field = batch
                        .column(batch.schema().index_of(stringify!($field)).unwrap())
                        .as_any()
                        .downcast_ref::<array_builder!($arrow_ty)>()
                        .unwrap();
                )*

                Ok((0..batch.num_rows())
                    .map(|i| $struct {
                        $(
                            $field: $field.value(i).into(),
                        )*
                    })
                    .collect())
            }

            fn to_record_batch(&self) -> Result<::duckdb::arrow::array::RecordBatch, ::cdktr_core::exceptions::GenericError> {
                let schema = ::std::sync::Arc::new(::duckdb::arrow::datatypes::Schema::new(vec![
                    $(::duckdb::arrow::datatypes::Field::new(stringify!($field), ::duckdb::arrow::datatypes::DataType::$arrow_ty, false)),*
                ]));

                $(
                    let mut $field = <builder_path!($arrow_ty)>::new();
                )*

                for item in self {
                    $(
                        $field.append_value(item.$field.clone());
                    )*
                }

                let arrays = vec![
                    $(::std::sync::Arc::new($field.finish()) as _),*
                ];

                Ok(::duckdb::arrow::array::RecordBatch::try_new(schema, arrays).map_err(|e| {
                    ::cdktr_core::exceptions::GenericError::DBError(format!(
                        "Failed to create arrow record batch for db insertion. Orig error: {}",
                        e
                    ))
                })?)
            }
        }
    };
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

    pub async fn lock_inner_client(&self) -> MutexGuard<'_, Connection> {
        self.cnxn.lock().await
    }
}

fn gen_ddl<'a>(cnxn: &'a Connection) -> Result<(), GenericError> {
    for ddl_statement in ddl::DDL {
        cnxn.execute(ddl_statement, [])
            .map_err(|e| GenericError::DBQueryStatementError(e.to_string()))?;
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

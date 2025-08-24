use cdktr_core::{exceptions::GenericError, get_cdktr_setting};
use duckdb::{params, Connection, Result};
use std::env;

mod ddl;

pub fn get_db_client() -> Connection {
    let client = get_db_cnxn(false);
    gen_ddl(&client).expect("Critical - failed to generate db ddl");
    client
}

pub fn get_test_db_client() -> Connection {
    let client = get_db_cnxn(true);
    gen_ddl(&client).expect("Critical - failed to generate db ddl");
    client
}

fn get_db_cnxn(in_memory: bool) -> Connection {
    if in_memory {
        Connection::open_in_memory().expect("Failed to open in-memory duckdb connection")
    } else {
        let app_db_path = get_cdktr_setting!(CDKTR_DB_PATH);
        Connection::open(&app_db_path).expect(&format!(
            "No connectable database can be found at: {}",
            app_db_path
        ))
    }
}

fn gen_ddl<'a>(client: &'a Connection) -> Result<(), GenericError> {
    for ddl_statement in ddl::DDL {
        client
            .execute(ddl_statement, [])
            .map_err(|e| GenericError::DBError(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_db_client() {
        let cli = get_test_db_client();
        assert!(cli.execute("select 1", params![]).is_ok());
    }
}

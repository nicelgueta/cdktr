use std::env;
use cdktr_core::get_cdktr_setting;
use duckdb::{params, Result, Connection};

fn get_db_client() -> Connection {
    get_db_cnxn(false)
}

fn get_test_db_client() -> Connection {
    get_db_cnxn(true)
}

fn get_db_cnxn(in_memory: bool) -> Connection {
    if in_memory {
        Connection::open_in_memory().expect("Failed to open in-memory duckdb connection")
    } else {
        let app_db_path = get_cdktr_setting!(CDKTR_DB_PATH);
        Connection::open(&app_db_path).expect(
            &format!("No connectable database can be found at: {}", app_db_path)
        )
    }
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

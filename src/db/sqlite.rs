// SQLite adapter via rusqlite (bundled).
// Opens DB files directly — no container/network needed.
use std::io::{Error, ErrorKind};

use crate::types::GenericCredentials;

/// Open a SQLite connection from the database path stored in credentials.
pub fn open_db(creds: &GenericCredentials) -> std::io::Result<rusqlite::Connection> {
    rusqlite::Connection::open(&creds.database)
        .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite open error: {}", e)))
}

/// List all user tables via sqlite_master.
pub fn list_tables(creds: &GenericCredentials) -> std::io::Result<String> {
    let conn = open_db(creds)?;
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite list_tables: {}", e)))?;

    let rows: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite list_tables query: {}", e)))?
        .filter_map(|r| r.ok())
        .collect();

    if rows.is_empty() {
        return Ok(String::from("(No tables found)"));
    }

    Ok(rows.join("\n"))
}

/// Execute an arbitrary SQL query against the SQLite database.
pub fn make_query(creds: &GenericCredentials, query: &str) -> std::io::Result<String> {
    let conn = open_db(creds)?;
    let mut stmt = conn
        .prepare(query)
        .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite query error: {}", e)))?;

    let column_count = stmt.column_count();
    let mut output = String::new();

    // Header
    let headers: Vec<String> = (0..column_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();
    output.push_str(&headers.join("|"));
    output.push('\n');

    // Separator
    let sep: String = headers.iter().map(|_| "-").collect::<Vec<_>>().join("+");
    output.push_str(&sep);
    output.push('\n');

    // Rows
    let rows = stmt
        .query_map([], |row| {
            let values: Vec<String> = (0..column_count)
                .map(|i| {
                    row.get::<_, rusqlite::types::Value>(i)
                        .map(|v| format!("{:?}", v))
                        .unwrap_or_else(|_| "NULL".to_string())
                })
                .collect();
            Ok(values.join("|"))
        })
        .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite query execution: {}", e)))?;

    for row in rows {
        let line =
            row.map_err(|e| Error::new(ErrorKind::Other, format!("SQLite row read: {}", e)))?;
        output.push_str(&line);
        output.push('\n');
    }

    Ok(output)
}

/// Export a table to CSV.
pub fn export_csv(creds: &GenericCredentials, table: &str, file_path: &str) -> std::io::Result<()> {
    let conn = open_db(creds)?;

    // Fetch all rows
    let query = format!("SELECT * FROM \"{table}\"");
    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite export: {}", e)))?;

    let column_count = stmt.column_count();
    let headers: Vec<String> = (0..column_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();

    let rows_data: Vec<Vec<String>> = stmt
        .query_map([], |row| {
            let values: Vec<String> = (0..column_count)
                .map(|i| {
                    row.get::<_, rusqlite::types::Value>(i)
                        .map(|v| format!("{:?}", v))
                        .unwrap_or_else(|_| "NULL".to_string())
                })
                .collect();
            Ok(values)
        })
        .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite export query: {}", e)))?
        .filter_map(|r| r.ok())
        .collect();

    // Write CSV
    let mut wtr = csv::Writer::from_path(file_path)
        .map_err(|e| Error::new(ErrorKind::Other, format!("CSV writer: {}", e)))?;

    wtr.write_record(&headers)
        .map_err(|e| Error::new(ErrorKind::Other, format!("CSV headers: {}", e)))?;

    for row in &rows_data {
        wtr.write_record(row)
            .map_err(|e| Error::new(ErrorKind::Other, format!("CSV row: {}", e)))?;
    }

    wtr.flush()
        .map_err(|e| Error::new(ErrorKind::Other, format!("CSV flush: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DbType, GenericCredentials};
    use rusqlite::Connection;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn sqlite_creds(path: &str) -> GenericCredentials {
        GenericCredentials {
            db_type: DbType::Sqlite,
            host: String::new(),
            port: String::new(),
            user: String::new(),
            password: String::new(),
            database: path.to_string(),
        }
    }

    fn unique_test_dir(prefix: &str) -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("dl_sqlite_{prefix}_{pid}_{id}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn setup_test_db() -> (std::path::PathBuf, GenericCredentials) {
        let dir = unique_test_dir("test_db");
        let db_path = dir.join("test.db");
        let creds = sqlite_creds(db_path.to_str().unwrap());

        let conn = Connection::open(&creds.database).unwrap();
        conn.execute_batch(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT);
             CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, total REAL);
             INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');
             INSERT INTO users VALUES (2, 'Bob', 'bob@example.com');
             INSERT INTO orders VALUES (1, 1, 99.99);",
        )
        .unwrap();

        (dir, creds)
    }

    #[test]
    fn test_sqlite_list_tables() {
        let (dir, creds) = setup_test_db();
        let result = list_tables(&creds).unwrap();
        assert!(
            result.contains("users"),
            "Should list users table, got: {}",
            result
        );
        assert!(
            result.contains("orders"),
            "Should list orders table, got: {}",
            result
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sqlite_list_tables_single_table() {
        let dir = unique_test_dir("single_table");
        let db_path = dir.join("empty.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE t (x INTEGER)").unwrap();
        drop(conn);

        let creds = sqlite_creds(db_path.to_str().unwrap());
        let result = list_tables(&creds).unwrap();
        assert_eq!(result, "t", "Should list the 't' table");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sqlite_make_query() {
        let (dir, creds) = setup_test_db();
        let result = make_query(&creds, "SELECT name FROM users ORDER BY id").unwrap();
        assert!(
            result.contains("Alice"),
            "Query result should contain Alice, got: {}",
            result
        );
        assert!(
            result.contains("Bob"),
            "Query result should contain Bob, got: {}",
            result
        );
        assert!(
            result.contains("name"),
            "Query result should contain header 'name'"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sqlite_make_query_count() {
        let (dir, creds) = setup_test_db();
        let result = make_query(&creds, "SELECT COUNT(*) AS cnt FROM users").unwrap();
        assert!(result.contains("2"), "Count should be 2, got: {}", result);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sqlite_export_csv() {
        let (dir, creds) = setup_test_db();
        let csv_path = std::env::temp_dir().join("dl_test_export_users.csv");
        let _ = fs::remove_file(&csv_path);

        export_csv(&creds, "users", csv_path.to_str().unwrap()).unwrap();

        let content = fs::read_to_string(&csv_path).unwrap();
        assert!(
            content.contains("id,name,email"),
            "CSV should contain headers"
        );
        assert!(content.contains("Alice"), "CSV should contain Alice");
        assert!(
            content.contains("bob@example.com"),
            "CSV should contain Bob's email"
        );
        let _ = fs::remove_file(&csv_path);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sqlite_export_csv_populated_table() {
        let (dir, creds) = setup_test_db();
        let csv_path = std::env::temp_dir().join("dl_test_export_orders.csv");
        let _ = fs::remove_file(&csv_path);

        export_csv(&creds, "orders", csv_path.to_str().unwrap()).unwrap();

        let content = fs::read_to_string(&csv_path).unwrap();
        assert!(
            content.contains("id,user_id,total"),
            "CSV should contain headers"
        );
        assert!(content.contains("99.99"), "CSV should contain order data");
        let _ = fs::remove_file(&csv_path);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sqlite_open_db_file_not_found() {
        let creds = sqlite_creds("/nonexistent/path/db.sqlite");
        let result = list_tables(&creds);
        assert!(result.is_err(), "Should fail for non-existent file");
    }
}

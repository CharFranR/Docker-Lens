// Multi-DB dispatch module.
// Routes `list_tables`, `make_query`, `export_csv`, and `inspect_schema`
// to engine-specific adapters based on `GenericCredentials.db_type`.
pub mod mongo;
pub mod mysql;
pub mod postgres;
pub mod sqlite;

use crate::types::{ColumnaInfo, DbType, GenericCredentials, TablaInfo};
use std::io::{Error, ErrorKind};

/// List all tables/collections for the given credentials.
pub fn list_tables(creds: &GenericCredentials) -> std::io::Result<String> {
    match creds.db_type {
        DbType::Postgres => postgres::list_tables(creds),
        DbType::Mysql | DbType::Mariadb => mysql::list_tables(creds),
        DbType::Sqlite => sqlite::list_tables(creds),
        DbType::Mongo => mongo::list_tables(creds),
    }
}

/// Execute an arbitrary query against the target database.
pub fn make_query(creds: &GenericCredentials, query: &str) -> std::io::Result<String> {
    match creds.db_type {
        DbType::Postgres => postgres::make_query(creds, query),
        DbType::Mysql | DbType::Mariadb => mysql::make_query(creds, query),
        DbType::Sqlite => sqlite::make_query(creds, query),
        DbType::Mongo => mongo::make_query(creds, query),
    }
}

/// Export a table to CSV at the given file path.
pub fn export_csv(creds: &GenericCredentials, table: &str, path: &str) -> std::io::Result<()> {
    match creds.db_type {
        DbType::Postgres => postgres::export_csv(creds, table, path),
        DbType::Mysql | DbType::Mariadb => mysql::export_csv(creds, table, path),
        DbType::Sqlite => sqlite::export_csv(creds, table, path),
        DbType::Mongo => mongo::export_csv(creds, table, path),
    }
}

/// Inspect database schema and return structured table/column info.
/// PostgreSQL and MySQL/MariaDB use information_schema.columns.
/// SQLite uses PRAGMA table_info.
/// MongoDB infers schema from sampling documents.
pub fn inspect_schema(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    match creds.db_type {
        DbType::Postgres => inspect_schema_pg(creds),
        DbType::Mysql | DbType::Mariadb => inspect_schema_mysql(creds),
        DbType::Sqlite => inspect_schema_sqlite(creds),
        DbType::Mongo => inspect_schema_mongo(creds),
    }
}

/// PostgreSQL schema inspection via information_schema.columns.
fn inspect_schema_pg(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    // Get table names
    let tables_query = "SELECT tablename FROM pg_tables WHERE schemaname = 'public';";
    let raw_tables = postgres::make_query(creds, tables_query)?;

    let table_names: Vec<String> = raw_tables
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with("tablename")
                && !l.starts_with('-')
                && !l.starts_with('(')
                && !l.contains("rows)")
        })
        .collect();

    if table_names.is_empty() {
        return Ok(Vec::new());
    }

    let mut raw_structures = Vec::new();
    for table_name in &table_names {
        let query = format!(
            "SELECT column_name, data_type, is_nullable, column_default \
             FROM information_schema.columns \
             WHERE table_name = '{}' \
             ORDER BY ordinal_position;",
            table_name
        );
        let result = postgres::make_query(creds, &query)?;
        raw_structures.push(result);
    }

    Ok(postgres::parse_db_structure(&raw_structures, &table_names))
}

/// SQLite schema inspection via PRAGMA table_info.
fn inspect_schema_sqlite(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    let conn = sqlite::open_db(creds)?;

    // Get user table names
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite schema: {}", e)))?;

    let table_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite schema query: {}", e)))?
        .filter_map(|r| r.ok())
        .collect();

    let mut tables = Vec::new();

    for name in &table_names {
        let pragma_query = format!("PRAGMA table_info(\"{}\")", name);
        let mut pragma_stmt = conn
            .prepare(&pragma_query)
            .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite PRAGMA: {}", e)))?;

        let mut columns = Vec::new();
        let rows = pragma_stmt
            .query_map([], |row| {
                let col_name: String = row.get(1)?;
                let col_type: String = row.get(2)?;
                let not_null: bool = row.get(3)?;
                let default: Option<String> = row.get(4)?;
                Ok((col_name, col_type, not_null, default))
            })
            .map_err(|e| Error::new(ErrorKind::Other, format!("SQLite PRAGMA query: {}", e)))?;

        for row in rows {
            let (col_name, col_type, not_null, default) =
                row.map_err(|e| Error::new(ErrorKind::Other, format!("SQLite row: {}", e)))?;
            columns.push(ColumnaInfo {
                nombre: col_name,
                tipo: col_type,
                nullable: if not_null {
                    "NO".to_string()
                } else {
                    "YES".to_string()
                },
                default,
            });
        }

        if !columns.is_empty() {
            tables.push(TablaInfo {
                nombre: name.clone(),
                columnas: columns,
            });
        }
    }

    Ok(tables)
}

/// MySQL/MariaDB schema inspection via information_schema.columns.
fn inspect_schema_mysql(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    let tables_query = "SHOW TABLES;";
    let raw_tables = mysql::make_query(creds, tables_query)?;

    let table_names: Vec<String> = raw_tables
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with("Tables_in_")
                && !l.starts_with('+')
                && !l.starts_with('|')
                && !l.contains("rows in set")
        })
        .filter(|l| !l.is_empty())
        .collect();

    if table_names.is_empty() {
        return Ok(Vec::new());
    }

    let mut raw_structures = Vec::new();
    for table_name in &table_names {
        let query = format!(
            "SELECT column_name, data_type, is_nullable, column_default \
             FROM information_schema.columns \
             WHERE table_name = '{}' AND table_schema = '{}' \
             ORDER BY ordinal_position;",
            table_name, creds.database
        );
        let result = mysql::make_query(creds, &query)?;
        raw_structures.push(result);
    }

    Ok(postgres::parse_db_structure(&raw_structures, &table_names))
}

/// MongoDB schema inspection by sampling first document from each collection.
fn inspect_schema_mongo(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    // Get collection names
    let coll_names_raw = mongo::list_tables(creds)?;
    if coll_names_raw.contains("No collections found") || coll_names_raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    let coll_names: Vec<String> = coll_names_raw.lines().map(|l| l.trim().to_string()).collect();

    let mut tables = Vec::new();

    for coll_name in &coll_names {
        // Query one document to infer fields
        let query = format!(
            r#"{{"find": "{}", "limit": 1}}"#,
            coll_name
        );
        let result = mongo::make_query(creds, &query)?;

        if result.contains("No documents found") || result.trim().is_empty() {
            tables.push(TablaInfo {
                nombre: coll_name.clone(),
                columnas: Vec::new(),
            });
            continue;
        }

        // Parse the tab-separated output to infer column types
        let columns = parse_mongo_fields_from_output(&result);
        tables.push(TablaInfo {
            nombre: coll_name.clone(),
            columnas: columns,
        });
    }

    Ok(tables)
}

/// Parse MongoDB docs_to_string output to infer field names and types.
fn parse_mongo_fields_from_output(output: &str) -> Vec<ColumnaInfo> {
    let mut lines = output.lines();
    let header_line = lines.next().unwrap_or("");
    // Skip separator line
    let _sep = lines.next();
    let first_row = lines.next().unwrap_or("");

    let headers: Vec<&str> = header_line.split('\t').map(|s| s.trim()).collect();
    let values: Vec<&str> = first_row.split('\t').map(|s| s.trim()).collect();

    headers
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let val = values.get(i).copied().unwrap_or("");
            let inferred_type = infer_mongo_type(val);
            ColumnaInfo {
                nombre: name.to_string(),
                tipo: inferred_type,
                nullable: "YES".to_string(),
                default: None,
            }
        })
        .collect()
}

/// Infer a SQL-like type from a MongoDB value string.
fn infer_mongo_type(val: &str) -> String {
    if val.is_empty() || val == "null" {
        return "text".to_string();
    }
    if val == "true" || val == "false" {
        return "boolean".to_string();
    }
    if val.parse::<i64>().is_ok() {
        return "integer".to_string();
    }
    if val.parse::<f64>().is_ok() {
        return "double".to_string();
    }
    "text".to_string()
}

/// Export any supported database to SQLite (cross-DB migration).
/// Routes to the appropriate source-adapter export logic.
pub fn export_to_sqlite(creds: &GenericCredentials, sqlite_path: &str) -> std::io::Result<()> {
    match creds.db_type {
        DbType::Postgres => postgres::export_to_sqlite(creds, sqlite_path),
        DbType::Mysql | DbType::Mariadb => export_mysql_to_sqlite(creds, sqlite_path),
        DbType::Sqlite => Err(Error::new(
            ErrorKind::InvalidInput,
            "Source database is already SQLite — use file copy instead",
        )),
        DbType::Mongo => export_mongo_to_sqlite(creds, sqlite_path),
    }
}

/// Export MySQL/MariaDB to SQLite.
fn export_mysql_to_sqlite(creds: &GenericCredentials, sqlite_path: &str) -> std::io::Result<()> {
    let tables_query = "SHOW TABLES;";
    let raw_tables = mysql::make_query(creds, tables_query)?;

    let table_names: Vec<String> = raw_tables
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with("Tables_in_")
                && !l.starts_with('+')
                && !l.starts_with('|')
        })
        .collect();

    if table_names.is_empty() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "No tables found in MySQL/MariaDB database",
        ));
    }

    let mut raw_structures = Vec::new();
    for table_name in &table_names {
        let query = format!(
            "SELECT column_name, data_type, is_nullable, column_default \
             FROM information_schema.columns \
             WHERE table_name = '{}' AND table_schema = '{}' \
             ORDER BY ordinal_position;",
            table_name, creds.database
        );
        let result = mysql::make_query(creds, &query)?;
        raw_structures.push(result);
    }

    let db_tables = postgres::parse_db_structure(&raw_structures, &table_names);
    let schema = postgres::convert_to_sqlite_schema(&db_tables);

    let conn = rusqlite::Connection::open(sqlite_path)
        .map_err(|e| Error::new(ErrorKind::Other, format!("Error creating SQLite: {e}")))?;

    for table in &schema.tables {
        let create_sql = postgres::generate_create_table(table);
        conn.execute_batch(&create_sql).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Error creating table '{}': {e}", table.name),
            )
        })?;
    }

    for table_name in &table_names {
        let temp_csv = format!("/tmp/dl_export_{}.csv", table_name);
        mysql::export_csv(creds, table_name, &temp_csv)?;

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(&temp_csv)
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Error reading CSV for '{}': {e}", table_name),
                )
            })?;

        let headers: Vec<String> = rdr
            .headers()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Error reading headers: {e}")))?
            .iter()
            .map(|h| h.to_string())
            .collect();

        let placeholders: Vec<String> = (0..headers.len()).map(|_| "?".to_string()).collect();
        let insert_sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            table_name,
            headers
                .iter()
                .map(|h| format!("\"{}\"", h))
                .collect::<Vec<_>>()
                .join(", "),
            placeholders.join(", ")
        );

        let tx = conn.unchecked_transaction().map_err(|e| {
            Error::new(ErrorKind::Other, format!("Error starting transaction: {e}"))
        })?;

        {
            let mut stmt = tx.prepare(&insert_sql).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Error preparing INSERT for '{}': {e}", table_name),
                )
            })?;

            for result in rdr.records() {
                let record = result.map_err(|e| {
                    Error::new(ErrorKind::Other, format!("Error reading record: {e}"))
                })?;
                let values: Vec<&str> = record.iter().collect();
                stmt.execute(rusqlite::params_from_iter(values.iter()))
                    .map_err(|e| {
                        Error::new(
                            ErrorKind::Other,
                            format!("Error inserting into '{}': {e}", table_name),
                        )
                    })?;
            }
        }

        tx.commit()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Error on commit: {e}")))?;

        let _ = std::fs::remove_file(&temp_csv);
    }

    Ok(())
}

/// Export MongoDB to SQLite by sampling each collection.
fn export_mongo_to_sqlite(creds: &GenericCredentials, sqlite_path: &str) -> std::io::Result<()> {
    let coll_names_raw = mongo::list_tables(creds)?;
    if coll_names_raw.contains("No collections found") || coll_names_raw.trim().is_empty() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "No collections found in MongoDB",
        ));
    }

    let coll_names: Vec<String> = coll_names_raw.lines().map(|l| l.trim().to_string()).collect();

    let conn = rusqlite::Connection::open(sqlite_path)
        .map_err(|e| Error::new(ErrorKind::Other, format!("Error creating SQLite: {e}")))?;

    for coll_name in &coll_names {
        let temp_csv = format!("/tmp/dl_mongo_export_{}.csv", coll_name.replace(' ', "_"));
        mongo::export_csv(creds, coll_name, &temp_csv)?;

        // Read CSV to infer schema and create table
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(&temp_csv)
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Error reading CSV for '{}': {e}", coll_name),
                )
            })?;

        let headers: Vec<String> = rdr
            .headers()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Error reading headers: {e}")))?
            .iter()
            .map(|h| h.to_string())
            .collect();

        if headers.is_empty() {
            let _ = std::fs::remove_file(&temp_csv);
            continue;
        }

        // Create table with all TEXT columns (safe for MongoDB's flexible schema)
        let col_defs: Vec<String> = headers
            .iter()
            .map(|h| format!("\"{}\" TEXT", h))
            .collect();
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" (\n    {}\n);",
            coll_name,
            col_defs.join(",\n    ")
        );

        conn.execute_batch(&create_sql).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Error creating table '{}': {e}", coll_name),
            )
        })?;

        let placeholders: Vec<String> = (0..headers.len()).map(|_| "?".to_string()).collect();
        let insert_sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            coll_name,
            headers
                .iter()
                .map(|h| format!("\"{}\"", h))
                .collect::<Vec<_>>()
                .join(", "),
            placeholders.join(", ")
        );

        let tx = conn.unchecked_transaction().map_err(|e| {
            Error::new(ErrorKind::Other, format!("Error starting transaction: {e}"))
        })?;

        {
            let mut stmt = tx.prepare(&insert_sql).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Error preparing INSERT for '{}': {e}", coll_name),
                )
            })?;

            for result in rdr.records() {
                let record = result.map_err(|e| {
                    Error::new(ErrorKind::Other, format!("Error reading record: {e}"))
                })?;
                let values: Vec<&str> = record.iter().collect();
                stmt.execute(rusqlite::params_from_iter(values.iter()))
                    .map_err(|e| {
                        Error::new(
                            ErrorKind::Other,
                            format!("Error inserting into '{}': {e}", coll_name),
                        )
                    })?;
            }
        }

        tx.commit()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Error on commit: {e}")))?;

        let _ = std::fs::remove_file(&temp_csv);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn pg_creds() -> GenericCredentials {
        GenericCredentials {
            db_type: DbType::Postgres,
            host: "localhost".into(),
            port: "5432".into(),
            user: "postgres".into(),
            password: "secret".into(),
            database: "appdb".into(),
        }
    }

    fn sqlite_test_creds() -> (std::path::PathBuf, GenericCredentials) {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("dl_dispatch_test_{}_{}", std::process::id(), id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
             INSERT INTO users VALUES (1, 'Alice');",
        )
        .unwrap();
        let creds = GenericCredentials {
            db_type: DbType::Sqlite,
            host: String::new(),
            port: String::new(),
            user: String::new(),
            password: String::new(),
            database: db_path.to_str().unwrap().to_string(),
        };
        (dir, creds)
    }

    fn mysql_creds() -> GenericCredentials {
        GenericCredentials {
            db_type: DbType::Mysql,
            host: "localhost".into(),
            port: "3306".into(),
            user: "root".into(),
            password: "root".into(),
            database: "mydb".into(),
        }
    }

    fn mongo_creds() -> GenericCredentials {
        GenericCredentials {
            db_type: DbType::Mongo,
            host: "localhost".into(),
            port: "27017".into(),
            user: "admin".into(),
            password: "admin".into(),
            database: "testdb".into(),
        }
    }

    // ── Dispatch routing tests ─────────────────────────────────────────

    #[test]
    fn test_dispatch_list_tables_postgres_routes_correctly() {
        let creds = pg_creds();
        let result = list_tables(&creds);
        match result {
            Ok(_) => {}
            Err(e) => assert_ne!(
                e.kind(),
                ErrorKind::Unsupported,
                "Postgres should NOT be Unsupported — dispatch routing failed"
            ),
        }
    }

    #[test]
    fn test_dispatch_list_tables_sqlite_routes_correctly() {
        let (dir, creds) = sqlite_test_creds();
        let result = list_tables(&creds);
        assert!(
            result.is_ok(),
            "Sqlite list_tables should succeed: {:?}",
            result.err()
        );
        assert!(result.unwrap().contains("users"), "Should list users table");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dispatch_list_tables_unsupported() {
        // All db_types are now supported. Test that a garbage type won't compile
        // by verifying that the match is exhaustive — this test is a compile-time check.
        // At runtime, all valid DbType variants route to an adapter.
        let creds = mysql_creds();
        let result = list_tables(&creds);
        // MySQL routes to mysql adapter which will fail with NotFound (no mysql binary)
        // but NOT with Unsupported
        match result {
            Ok(_) => {} // MySQL available
            Err(e) => assert_ne!(
                e.kind(),
                ErrorKind::Unsupported,
                "MySQL should route, not be Unsupported"
            ),
        }
    }

    #[test]
    fn test_dispatch_make_query_unsupported() {
        let creds = mysql_creds();
        let result = make_query(&creds, "SELECT 1");
        match result {
            Ok(_) => {}
            Err(e) => assert_ne!(
                e.kind(),
                ErrorKind::Unsupported,
                "MySQL make_query should route, not be Unsupported"
            ),
        }
    }

    #[test]
    fn test_dispatch_export_csv_unsupported() {
        let creds = mysql_creds();
        let result = export_csv(&creds, "users", "/tmp/test.csv");
        match result {
            Ok(_) => {}
            Err(e) => assert_ne!(
                e.kind(),
                ErrorKind::Unsupported,
                "MySQL export_csv should route, not be Unsupported"
            ),
        }
    }

    #[test]
    fn test_dispatch_make_query_postgres_routes_correctly() {
        let creds = pg_creds();
        let result = make_query(&creds, "SELECT 1");
        match result {
            Ok(_) => {}
            Err(e) => assert_ne!(
                e.kind(),
                ErrorKind::Unsupported,
                "Postgres should NOT be Unsupported — dispatch routing failed"
            ),
        }
    }

    #[test]
    fn test_dispatch_make_query_sqlite_routes_correctly() {
        let (dir, creds) = sqlite_test_creds();
        let result = make_query(&creds, "SELECT name FROM users");
        assert!(
            result.is_ok(),
            "Sqlite make_query should succeed: {:?}",
            result.err()
        );
        assert!(result.unwrap().contains("Alice"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dispatch_export_csv_postgres_routes_correctly() {
        let creds = pg_creds();
        let result = export_csv(&creds, "users", "/tmp/test.csv");
        match result {
            Ok(_) => {}
            Err(e) => assert_ne!(
                e.kind(),
                ErrorKind::Unsupported,
                "Postgres should NOT be Unsupported — dispatch routing failed"
            ),
        }
    }

    #[test]
    fn test_dispatch_export_csv_sqlite_routes_correctly() {
        let (dir, creds) = sqlite_test_creds();
        let csv_path = std::env::temp_dir().join("dl_dispatch_export.csv");
        let _ = fs::remove_file(&csv_path);
        let result = export_csv(&creds, "users", csv_path.to_str().unwrap());
        assert!(
            result.is_ok(),
            "Sqlite export_csv should succeed: {:?}",
            result.err()
        );
        let content = fs::read_to_string(&csv_path).unwrap();
        assert!(content.contains("Alice"));
        let _ = fs::remove_file(&csv_path);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dispatch_list_tables_mysql_routes_correctly() {
        let creds = mysql_creds();
        let result = list_tables(&creds);
        match result {
            Ok(_) => {} // MySQL available
            Err(e) => assert_ne!(
                e.kind(),
                ErrorKind::Unsupported,
                "MySQL should NOT be Unsupported — dispatch routing failed: {}",
                e
            ),
        }
    }

    #[test]
    fn test_dispatch_list_tables_mongo_routes_correctly() {
        let creds = mongo_creds();
        let result = list_tables(&creds);
        match result {
            Ok(_) => {} // MongoDB available
            Err(e) => assert_ne!(
                e.kind(),
                ErrorKind::Unsupported,
                "Mongo should NOT be Unsupported — dispatch routing failed: {}",
                e
            ),
        }
    }

    #[test]
    fn test_dispatch_make_query_mysql_routes_correctly() {
        let creds = mysql_creds();
        let result = make_query(&creds, "SELECT 1");
        match result {
            Ok(_) => {}
            Err(e) => assert_ne!(e.kind(), ErrorKind::Unsupported),
        }
    }

    #[test]
    fn test_dispatch_make_query_mongo_routes_correctly() {
        let creds = mongo_creds();
        // Invalid JSON for MongoDB should NOT be Unsupported, should be InvalidInput
        let result = make_query(&creds, "not json");
        match result {
            Ok(_) => {}
            Err(e) => {
                assert_ne!(
                    e.kind(),
                    ErrorKind::Unsupported,
                    "Mongo make_query should route, got: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_dispatch_all_types_route() {
        // All five DB types should route without ErrorKind::Unsupported
        let types = [
            (DbType::Postgres, pg_creds()),
            (DbType::Mysql, mysql_creds()),
            (DbType::Mariadb, GenericCredentials {
                db_type: DbType::Mariadb,
                host: "localhost".into(),
                port: "3306".into(),
                user: "root".into(),
                password: "root".into(),
                database: "mydb".into(),
            }),
            (DbType::Mongo, mongo_creds()),
            (DbType::Sqlite, {
                // Sqlite needs a real file
                let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
                let dir = std::env::temp_dir()
                    .join(format!("dl_alltypes_{}_{}", std::process::id(), id));
                let _ = fs::remove_dir_all(&dir);
                fs::create_dir_all(&dir).unwrap();
                let db_path = dir.join("test.db");
                rusqlite::Connection::open(&db_path).unwrap();
                GenericCredentials {
                    db_type: DbType::Sqlite,
                    host: String::new(),
                    port: String::new(),
                    user: String::new(),
                    password: String::new(),
                    database: db_path.to_str().unwrap().to_string(),
                }
            }),
        ];

        for (db_type, creds) in &types {
            let r = list_tables(creds);
            match r {
                Ok(_) => {}
                Err(e) => assert_ne!(
                    e.kind(),
                    ErrorKind::Unsupported,
                    "{:?} list_tables should NOT be Unsupported",
                    db_type
                ),
            }
        }

        // Cleanup sqlite temp dirs — best effort
        let last_id = TEST_COUNTER.load(Ordering::Relaxed);
        let dir = std::env::temp_dir()
            .join(format!("dl_alltypes_{}_{}", std::process::id(), last_id));
        let _ = fs::remove_dir_all(dir);
    }

    // ── Schema inspection tests ────────────────────────────────────────

    #[test]
    fn test_inspect_schema_sqlite() {
        let (dir, creds) = sqlite_test_creds();
        let tables = inspect_schema(&creds).unwrap();
        assert_eq!(tables.len(), 1, "Should have 1 table");
        assert_eq!(tables[0].nombre, "users");
        assert_eq!(tables[0].columnas.len(), 2);
        assert_eq!(tables[0].columnas[0].nombre, "id");
        assert_eq!(tables[0].columnas[0].tipo.to_uppercase(), "INTEGER");
        assert_eq!(tables[0].columnas[1].nombre, "name");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_inspect_schema_all_types_route() {
        // All DB types should route inspect_schema without ErrorKind::Unsupported
        let creds = mysql_creds();
        let result = inspect_schema(&creds);
        match result {
            Ok(_) => {}
            Err(e) => assert_ne!(
                e.kind(),
                ErrorKind::Unsupported,
                "MySQL inspect_schema should route: {}",
                e
            ),
        }

        let creds = mongo_creds();
        let result = inspect_schema(&creds);
        match result {
            Ok(_) => {}
            Err(e) => assert_ne!(
                e.kind(),
                ErrorKind::Unsupported,
                "Mongo inspect_schema should route: {}",
                e
            ),
        }
    }
}

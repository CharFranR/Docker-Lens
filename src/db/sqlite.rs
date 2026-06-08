// SQLite adapter via rusqlite (bundled).
// Opens DB files directly — no container/network needed.
use std::io::{Error, ErrorKind};

use crate::types::{GenericCredentials, ColumnaInfo, TablaInfo};

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


/// SQLite schema inspection via PRAGMA table_info.
pub fn inspect_schema_sqlite(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    let conn = open_db(creds)?;

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
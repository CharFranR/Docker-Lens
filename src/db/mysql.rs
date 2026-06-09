// MySQL/MariaDB adapter via mysql CLI.
// Same pattern as postgres.rs but uses `mysql` binary.

use std::io::{Error, ErrorKind};
use std::process::Command;

use crate::db::postgres;
use crate::types::{GenericCredentials, TablaInfo};


/// List all tables via `SHOW TABLES`.
pub fn list_tables(credentials: &GenericCredentials) -> std::io::Result<String> {
    let mut cmd = build_mysql_command(credentials);
    cmd.arg("-e").arg("SHOW TABLES;");

    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            return Err(Error::new(ErrorKind::Other, stderr.to_string()));
        }
        return Ok(String::from("(No tables found)"));
    }

    Ok(stdout)
}

/// Execute an arbitrary SQL query via mysql CLI.
pub fn make_query(credentials: &GenericCredentials, query: &str) -> std::io::Result<String> {
    let mut cmd = build_mysql_command(credentials);
    cmd.arg("-e").arg(query);

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::new(ErrorKind::Other, stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

/// MySQL/MariaDB schema inspection via information_schema.columns.
pub fn inspect_schema_mysql(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    let tables_query = "SHOW TABLES;";
    let raw_tables = make_query(creds, tables_query)?;

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
        let result = make_query(creds, &query)?;
        raw_structures.push(result);
    }

    Ok(postgres::parse_db_structure(&raw_structures, &table_names))
}




/// Export a table to CSV via mysql CLI --batch mode.
pub fn export_csv(
    credentials: &GenericCredentials,
    table: &str,
    file_path: &str,
) -> std::io::Result<()> {
    let file = std::fs::File::create(file_path)?;

    let sql = format!("SELECT * FROM `{table}`");
    let mut child = build_mysql_command(credentials)
        .arg("--batch")
        .arg("-e")
        .arg(&sql)
        .stdout(file)
        .spawn()?;

    let status = child.wait()?;

    if !status.success() {
        return Err(Error::new(ErrorKind::Other, "Error exporting CSV from MySQL"));
    }

    Ok(())
}

/// Build the base `mysql` Command with connection flags.
fn build_mysql_command(credentials: &GenericCredentials) -> Command {
    let mut cmd = Command::new("mysql");

    cmd.arg("-h")
        .arg(&credentials.host)
        .arg("-P")
        .arg(&credentials.port)
        .arg("-u")
        .arg(&credentials.user);

    // Pass password via env var instead of -p flag (avoids security warning)
    if !credentials.password.is_empty() {
        cmd.env("MYSQL_PWD", &credentials.password);
        // Skip the interactive password prompt
        cmd.arg("--skip-password-prompt");
    }

    if !credentials.database.is_empty() {
        cmd.arg("-D").arg(&credentials.database);
    }

    cmd
}


/// Export MySQL/MariaDB to SQLite.
pub fn export_mysql_to_sqlite(creds: &GenericCredentials, sqlite_path: &str) -> std::io::Result<()> {
    let tables_query = "SHOW TABLES;";
    let raw_tables = make_query(creds, tables_query)?;

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
        let result = make_query(creds, &query)?;
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
        export_csv(creds, table_name, &temp_csv)?;

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
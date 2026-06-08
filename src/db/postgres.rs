// PostgreSQL adapter via psql CLI.
// Extracted from src/psql.rs — adapted to GenericCredentials.
use std::io::{Error, ErrorKind};
use std::process::Command;

use rusqlite::Connection;

use crate::types::{
    ColumnaInfo, GenericCredentials, SQLiteColumn, SQLiteSchema, SQLiteTable, TablaInfo,
};

/// Resolve the Docker container IP for a given service name.
pub fn get_container_ip(service_winner: &str) -> Option<String> {
    let output = match Command::new("docker")
        .args(["ps", "-a", "--format", "{{.ID}}|{{.Names}}"])
        .output()
    {
        Ok(o) => o,
        Err(_) => {
            eprintln!("Error: Docker is not installed or not in PATH.");
            return None;
        }
    };

    let output_str = String::from_utf8_lossy(&output.stdout);

    for line in output_str.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            let container_id = parts[0];
            let container_name = parts[1];
            let name_clean = container_name.trim_start_matches('/');

            if name_clean.contains(service_winner) || container_name.contains(service_winner) {
                let inspect_output = match Command::new("docker")
                    .args([
                        "inspect",
                        container_id,
                        "--format",
                        "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}",
                    ])
                    .output()
                {
                    Ok(o) => o,
                    Err(_) => {
                        eprintln!("Error: Could not inspect container '{}'.", container_name);
                        return None;
                    }
                };

                let ip = String::from_utf8_lossy(&inspect_output.stdout)
                    .trim()
                    .to_string();
                return Some(ip);
            }
        }
    }

    None
}

/// List all tables via psql `\dt`.
pub fn list_tables(credentials: &GenericCredentials) -> std::io::Result<String> {
    let port = String::from("-p") + &credentials.port;
    let user = String::from("-U") + &credentials.user;
    let password = &credentials.password;
    let db = String::from("-d") + &credentials.database;

    let output = Command::new("psql")
        .args([
            "-hlocalhost",
            port.as_str(),
            user.as_str(),
            db.as_str(),
            "-c",
            "\\dt",
            "-w", // No pedir password
        ])
        .env("PGPASSWORD", password.as_str())
        .output()?;

    let all_tables = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(all_tables)
}

/// Execute an arbitrary SQL query via psql.
pub fn make_query(credentials: &GenericCredentials, query: &str) -> std::io::Result<String> {
    let port = String::from("-p") + &credentials.port;
    let user = String::from("-U") + &credentials.user;
    let password = &credentials.password;
    let db = String::from("-d") + &credentials.database;

    let response = Command::new("psql")
        .args([
            "-hlocalhost",
            port.as_str(),
            user.as_str(),
            db.as_str(),
            "-c",
            query,
            "-w", // No pedir password
        ])
        .env("PGPASSWORD", password.as_str())
        .output()?;

    if !response.status.success() {
        let stderr = String::from_utf8_lossy(&response.stderr);
        return Err(Error::new(ErrorKind::Other, stderr.to_string()));
    }

    let output = String::from_utf8_lossy(&response.stdout).to_string();

    Ok(output)
}


/// PostgreSQL schema inspection via information_schema.columns.
pub fn inspect_schema_pg(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    // Get table names
    let tables_query = "SELECT tablename FROM pg_tables WHERE schemaname = 'public';";
    let raw_tables = make_query(creds, tables_query)?;

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
        let result = make_query(creds, &query)?;
        raw_structures.push(result);
    }

    Ok(parse_db_structure(&raw_structures, &table_names))
}




/// Export a table to CSV via psql COPY.
pub fn export_csv(
    credentials: &GenericCredentials,
    table: &str,
    file_path: &str,
) -> std::io::Result<()> {
    let file = std::fs::File::create(file_path)?;
    let port = format!("-p{}", credentials.port);
    let user = format!("-U{}", credentials.user);
    let db = format!("-d{}", credentials.database);

    let sql = format!("COPY (SELECT * FROM {table}) TO STDOUT WITH CSV HEADER");
    let mut child = Command::new("psql")
        .args(["-hlocalhost", &port, &user, &db, "-c", &sql, "-w"])
        .env("PGPASSWORD", &credentials.password)
        .stdout(file)
        .spawn()?;

    let status = child.wait()?;

    if !status.success() {
        return Err(Error::new(ErrorKind::Other, "Error al exportar CSV"));
    }

    Ok(())
}

// ── Schema helpers (pure functions, independently testable) ────────────────

/// Map a PostgreSQL type to its SQLite equivalent.
pub fn map_pg_type_to_sqlite(pg_type: &str) -> String {
    let normalized = pg_type.to_lowercase();

    if normalized.contains("int") || normalized.contains("serial") {
        return "INTEGER".to_string();
    }

    if normalized.contains("float")
        || normalized.contains("double")
        || normalized.contains("numeric")
        || normalized.contains("decimal")
        || normalized.contains("real")
    {
        return "REAL".to_string();
    }

    if normalized.contains("char") || normalized.contains("text") || normalized.contains("varchar")
    {
        return "TEXT".to_string();
    }

    if normalized.contains("bool") {
        return "INTEGER".to_string(); // SQLite usa 0/1 para booleanos
    }

    if normalized.contains("date")
        || normalized.contains("time")
        || normalized.contains("timestamp")
    {
        return "TEXT".to_string(); // SQLite no tiene tipo fecha nativo
    }

    if normalized.contains("json") {
        return "TEXT".to_string();
    }

    if normalized.contains("bytea") {
        return "BLOB".to_string();
    }

    if normalized.contains("uuid") {
        return "TEXT".to_string();
    }

    "TEXT".to_string()
}

/// Parse raw information_schema output into TablaInfo structs.
pub fn parse_db_structure(raw_tables: &[String], table_names: &[String]) -> Vec<TablaInfo> {
    let mut tables = Vec::new();

    for (idx, raw) in raw_tables.iter().enumerate() {
        let name = if idx < table_names.len() {
            table_names[idx].trim().to_string()
        } else {
            format!("table_{}", idx)
        };

        let mut columns = Vec::new();

        for line in raw.lines() {
            let line = line.trim();

            if line.is_empty()
                || line.starts_with("column_name")
                || line.starts_with('-')
                || line.starts_with('(')
                || line.contains("rows)")
            {
                continue;
            }

            let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();

            if parts.len() >= 3 {
                let col = ColumnaInfo {
                    nombre: parts[0].to_string(),
                    tipo: parts[1].to_string(),
                    nullable: parts[2].to_string(),
                    default: if parts.len() >= 4 && !parts[3].is_empty() {
                        Some(parts[3].to_string())
                    } else {
                        None
                    },
                };
                columns.push(col);
            }
        }

        if !columns.is_empty() {
            tables.push(TablaInfo {
                nombre: name,
                columnas: columns,
            });
        }
    }

    tables
}

/// Convert TablaInfo (PostgreSQL) to SQLiteTable (SQLite).
pub fn convert_to_sqlite_schema(tables: &[TablaInfo]) -> SQLiteSchema {
    let sqlite_tables: Vec<SQLiteTable> = tables
        .iter()
        .map(|t| {
            let columns: Vec<SQLiteColumn> = t
                .columnas
                .iter()
                .map(|c| SQLiteColumn {
                    name: c.nombre.clone(),
                    sqlite_type: map_pg_type_to_sqlite(&c.tipo),
                    nullable: c.nullable.to_uppercase() == "YES",
                    default: c.default.clone(),
                })
                .collect();

            SQLiteTable {
                name: t.nombre.clone(),
                columns,
            }
        })
        .collect();

    SQLiteSchema {
        tables: sqlite_tables,
    }
}

/// Generate CREATE TABLE statement for a SQLite table.
pub fn generate_create_table(table: &SQLiteTable) -> String {
    let mut sql = format!("CREATE TABLE IF NOT EXISTS \"{}\" (\n", table.name);

    let col_defs: Vec<String> = table
        .columns
        .iter()
        .map(|c| {
            let mut def = format!("    \"{}\" {}", c.name, c.sqlite_type);

            if !c.nullable {
                def.push_str(" NOT NULL");
            }

            if let Some(ref default_val) = c.default {
                if !default_val.contains('(') && !default_val.contains("::") {
                    def.push_str(&format!(" DEFAULT {}", default_val));
                }
            }

            def
        })
        .collect();

    sql.push_str(&col_defs.join(",\n"));
    sql.push_str("\n);");

    sql
}

/// Export a PostgreSQL database to SQLite.
///
/// Strategy:
/// 1. Get PostgreSQL schema
/// 2. Map types
/// 3. Create tables in SQLite
/// 4. Export data via temporary CSV
/// 5. Insert into SQLite with transactions
pub fn export_to_sqlite(
    credentials: &GenericCredentials,
    sqlite_path: &str,
) -> std::io::Result<()> {
    let tables_query = "SELECT tablename FROM pg_tables WHERE schemaname = 'public';";
    let raw_tables = make_query(credentials, tables_query)?;

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
        return Err(Error::new(
            ErrorKind::NotFound,
            "No se encontraron tablas en la base de datos",
        ));
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

        match make_query(credentials, &query) {
            Ok(value) => raw_structures.push(value),
            Err(e) => return Err(e),
        };
    }

    let db_tables = parse_db_structure(&raw_structures, &table_names);
    let schema = convert_to_sqlite_schema(&db_tables);

    let conn = Connection::open(sqlite_path)
        .map_err(|e| Error::new(ErrorKind::Other, format!("Error creando SQLite: {}", e)))?;

    for table in &schema.tables {
        let create_sql = generate_create_table(table);
        conn.execute_batch(&create_sql).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Error creando tabla '{}': {}", table.name, e),
            )
        })?;
    }

    for table_name in &table_names {
        let temp_csv = format!("/tmp/dl_export_{}.csv", table_name);
        export_csv(credentials, table_name, &temp_csv)?;

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(&temp_csv)
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Error leyendo CSV para '{}': {}", table_name, e),
                )
            })?;

        let headers: Vec<String> = rdr
            .headers()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Error leyendo headers: {}", e)))?
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
            Error::new(
                ErrorKind::Other,
                format!("Error iniciando transacción: {}", e),
            )
        })?;

        {
            let mut stmt = tx.prepare(&insert_sql).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Error preparando INSERT para '{}': {}", table_name, e),
                )
            })?;

            for result in rdr.records() {
                let record = result.map_err(|e| {
                    Error::new(ErrorKind::Other, format!("Error leyendo registro: {}", e))
                })?;

                let values: Vec<&str> = record.iter().collect();
                stmt.execute(rusqlite::params_from_iter(values.iter()))
                    .map_err(|e| {
                        Error::new(
                            ErrorKind::Other,
                            format!("Error insertando en '{}': {}", table_name, e),
                        )
                    })?;
            }
        }

        tx.commit()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Error en commit: {}", e)))?;

        let _ = std::fs::remove_file(&temp_csv);
    }

    Ok(())
}

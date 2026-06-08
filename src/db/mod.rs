// Multi-DB dispatch module.
// Routes `list_tables`, `make_query`, `export_csv`, and `inspect_schema`
// to engine-specific adapters based on `GenericCredentials.db_type`.
pub mod mongo;
pub mod mysql;
pub mod postgres;
pub mod sqlite;

use crate::types::{DbType, GenericCredentials, TablaInfo};
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

/// Get the Docker container IP for a given service name.
/// Only works for Postgres, MySQL, and MariaDB (CLI-based adapters).
/// Returns None for SQLite (file-based) and MongoDB (crate-based).
pub fn get_container_ip(creds: &GenericCredentials, service_name: &str) -> Option<String> {
    match creds.db_type {
        DbType::Postgres => postgres::get_container_ip(service_name),
        DbType::Mysql | DbType::Mariadb => mysql::get_container_ip(service_name),
        DbType::Sqlite | DbType::Mongo => None,
    }
}

/// Inspect database schema and return structured table/column info.
/// PostgreSQL and MySQL/MariaDB use information_schema.columns.
/// SQLite uses PRAGMA table_info.
/// MongoDB infers schema from sampling documents.
pub fn inspect_schema(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    match creds.db_type {
        DbType::Postgres => postgres::inspect_schema_pg(creds),
        DbType::Mysql | DbType::Mariadb => mysql::inspect_schema_mysql(creds),
        DbType::Sqlite => sqlite::inspect_schema_sqlite(creds),
        DbType::Mongo => mongo::inspect_schema_mongo(creds),
    }
}



/// Export any supported database to SQLite (cross-DB migration).
/// Routes to the appropriate source-adapter export logic.
pub fn export_to_sqlite(creds: &GenericCredentials, sqlite_path: &str) -> std::io::Result<()> {
    match creds.db_type {
        DbType::Postgres => postgres::export_to_sqlite(creds, sqlite_path),
        DbType::Mysql | DbType::Mariadb => mysql::export_mysql_to_sqlite(creds, sqlite_path),
        DbType::Sqlite => Err(Error::new(
            ErrorKind::InvalidInput,
            "Source database is already SQLite — use file copy instead",
        )),
        DbType::Mongo => mongo::export_mongo_to_sqlite(creds, sqlite_path),
    }
}
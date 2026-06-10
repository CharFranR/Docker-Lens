pub mod docker;
pub mod mongo;
pub mod mysql;
pub mod postgres;
pub mod sqlite;

use crate::compose::{serializer_docker, Service};
use crate::heuristic;
use crate::types::{DbType, GenericCredentials, TablaInfo};
use std::io::{Error, ErrorKind};


/// Extract a YAML env value by key from a mapping
pub(crate) fn yaml_env_get(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
    for (k, v) in map.iter() {
        if let Some(s) = k.as_str() {
            if s.eq_ignore_ascii_case(key) {
                return v.as_str().map(|s| s.to_string());
            }
        }
    }
    None
}

/// Extract an env var from a sequence of key-value strings
pub(crate) fn seq_env_get(seq: &[serde_yaml::Value], prefix: &str) -> Option<String> {
    for item in seq {
        if let Some(s) = item.as_str() {
            if s.to_ascii_lowercase()
                .starts_with(&prefix.to_ascii_lowercase())
            {
                if let Some(val) = s.splitn(2, '=').nth(1) {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

/// Extract string env var from either mapping or sequence on a service
pub(crate) fn extract_env(svc: &Service, key: &str) -> Option<String> {
    let env_val = svc.environment.as_ref()?;
    if let Some(map) = env_val.as_mapping() {
        yaml_env_get(map, key)
    } else if let Some(seq) = env_val.as_sequence() {
        seq_env_get(seq, key)
    } else {
        None
    }
}

/// List all tables/collections for the given credentials
pub fn list_tables(creds: &GenericCredentials) -> std::io::Result<String> {
    match creds.db_type {
        DbType::Postgres => postgres::list_tables(creds),
        DbType::Mysql | DbType::Mariadb => mysql::list_tables(creds),
        DbType::Sqlite => sqlite::list_tables(creds),
        DbType::Mongo => mongo::list_tables(creds),
    }
}

/// Execute an arbitrary query against the target database
pub fn make_query(creds: &GenericCredentials, query: &str) -> std::io::Result<String> {
    match creds.db_type {
        DbType::Postgres => postgres::make_query(creds, query),
        DbType::Mysql | DbType::Mariadb => mysql::make_query(creds, query),
        DbType::Sqlite => sqlite::make_query(creds, query),
        DbType::Mongo => mongo::make_query(creds, query),
    }
}

/// Export a table to CSV at the given file path
pub fn export_csv(creds: &GenericCredentials, table: &str, path: &str) -> std::io::Result<()> {
    match creds.db_type {
        DbType::Postgres => postgres::export_csv(creds, table, path),
        DbType::Mysql | DbType::Mariadb => mysql::export_csv(creds, table, path),
        DbType::Sqlite => sqlite::export_csv(creds, table, path),
        DbType::Mongo => Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "CSV export is not supported for MongoDB — use mongodump or mongoexport instead",
        )),
    }
}

/// Get the Docker container IP for a given service name
pub fn get_container_ip(creds: &GenericCredentials, service_name: &str) -> Option<String> {
    match creds.db_type {
        DbType::Postgres => docker::get_container_ip(service_name),
        DbType::Mysql | DbType::Mariadb => docker::get_container_ip(service_name),
        DbType::Sqlite | DbType::Mongo => None,
    }
}

/// Inspect database schema and return structured table/column info
pub fn inspect_schema(creds: &GenericCredentials) -> std::io::Result<Vec<TablaInfo>> {
    match creds.db_type {
        DbType::Postgres => postgres::inspect_schema_pg(creds),
        DbType::Mysql | DbType::Mariadb => mysql::inspect_schema_mysql(creds),
        DbType::Sqlite => sqlite::inspect_schema_sqlite(creds),
        DbType::Mongo => mongo::inspect_schema_mongo(creds),
    }
}

/// Export any supported database to SQLite
pub fn export_to_sqlite(creds: &GenericCredentials, sqlite_path: &str) -> std::io::Result<()> {
    match creds.db_type {
        DbType::Postgres => postgres::export_to_sqlite(creds, sqlite_path),
        DbType::Mysql | DbType::Mariadb => mysql::export_mysql_to_sqlite(creds, sqlite_path),
        DbType::Sqlite => Err(Error::new(
            ErrorKind::InvalidInput,
            "Source database is already SQLite — use file copy instead",
        )),
        DbType::Mongo => Err(Error::new(
            ErrorKind::Unsupported,
            "SQLite export is not supported for MongoDB — use mongodump or mongoexport instead",
        )),
    }
}


/// Detect and extract credentials from a docker-compose.yml.
pub fn credentials_from_compose(folder_path: &std::path::Path) -> std::io::Result<GenericCredentials> {
    let (db_type, service_name) = heuristic::find_db_service(&folder_path.to_path_buf())?;

    let compose_path = folder_path.join("docker-compose.yml");
    let text = std::fs::read_to_string(compose_path)?;
    let data = serializer_docker(text)?;

    let svc = data.services.get(&service_name).ok_or_else(|| {
        Error::new(ErrorKind::NotFound, format!("Service '{}' not found in compose", service_name))
    })?;

    Ok(match db_type {
        DbType::Postgres => postgres::extract_credentials(svc),
        DbType::Mysql | DbType::Mariadb => mysql::extract_credentials(svc, db_type),
        DbType::Mongo => mongo::extract_credentials(svc),
        DbType::Sqlite => sqlite::extract_credentials(svc),
    })
}
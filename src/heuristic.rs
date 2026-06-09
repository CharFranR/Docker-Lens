use std::collections::HashMap;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

use crate::compose::serializer_docker;
use crate::types::DbType;

/// Service name patterns that suggest a DB service 
pub const NAME_LIST: &[&str] = &[
    "db",
    "database",
    "postgres",
    "postgresql",
    "pg",
    "pgsql",
    "postgres_db",
    "postgres-database",
    "postgres_service",
    "postgres-server",
    "postgresdb",
    "db-server",
    "db_postgres",
    "database_postgres",
    "main_db",
    "primary_db",
    "data",
    "data_db",
    "storage_db",
    "storage",
    "dbs",
    "database_service",
    "sql_db",
    "sql_database",
    "pg-master",
    "pg-replica",
    "database-main",
    "db-main",
    "primary-database",
    // Multi-DB additions
    "mysql",
    "mariadb",
    "mysql-db",
    "mysql_db",
    "mysql-server",
    "mongo",
    "mongodb",
    "mongo-db",
    "mongo_db",
    "mongodb-server",
    "sqlite",
    "sqlite3",
    "sqlite-db",
    "sqlite_db",
] as &[&str];

/// Image patterns per DB type.
pub const PG_IMAGE_LIST: &[&str] = &[
    "postgres",
    "postgis",
    "postgresql",
    "timescaledb",
    "timescale",
    "bitnami/postgresql",
];

pub const MYSQL_IMAGE_LIST: &[&str] = &["mysql", "bitnami/mysql"];

pub const MARIADB_IMAGE_LIST: &[&str] = &["mariadb", "bitnami/mariadb"];

pub const MONGO_IMAGE_LIST: &[&str] = &["mongo", "bitnami/mongodb"];

pub const SQLITE_IMAGE_LIST: &[&str] = &["keinos/sqlite3", "sqlite3"];

/// Image score: 30 points for exact image match
const IMAGE_SCORE: i32 = 30;
/// Name match score: 10 points for service name matching DB patterns
const NAME_SCORE: i32 = 10;
/// Port match score: 25 points for standard DB port
const PORT_SCORE: i32 = 25;
/// Minimum score to consider a service as a database
const MIN_DB_SCORE: i32 = 30;

/// Env var score: 15-20 points per detected credential variable
const ENV_CREDENTIAL_SCORE: i32 = 15;
const ENV_DB_SCORE: i32 = 20;
/// No-depends-on bonus: 20 points (DBs are typically leaf services)
const NO_DEPENDS_BONUS: i32 = 20;

/// Per-DB port defaults
const PG_PORT: &str = "5432";
const MYSQL_PORT: &str = "3306";
const MYSQL_ALT_PORT: &str = "3307";
const MONGO_PORT: &str = "27017";

/// Score a single service against one DB type
fn score_service_for_db(
    svc: &crate::compose::Service,
    image_list: &[&str],
    port_defaults: &[&str],
    env_prefixes: &[&str],
) -> i32 {
    let mut score = 0;

    // No-depends-on bonus (leaf service)
    if svc.depends_on.is_none() {
        score += NO_DEPENDS_BONUS;
    }

    // Image match
    if let Some(img) = &svc.image {
        let img_norm = img.to_ascii_lowercase();
        if image_list.iter().any(|w| img_norm.contains(w)) {
            score += IMAGE_SCORE;
        }
    }

    // Port match (any of the default ports)
    if let Some(ports_vec) = &svc.ports {
        for p in ports_vec {
            let p_clean = p.split('/').next().unwrap_or(p);
            let parts: Vec<&str> = p_clean.split(':').collect();
            let port = if parts.len() == 1 {
                parts[0]
            } else if parts.len() == 2 {
                parts[1]
            } else {
                parts[2]
            };
            if port_defaults.contains(&port) {
                score += PORT_SCORE;
                break;
            }
        }
    }

    // Environment evidence
    if let Some(env_value) = &svc.environment {
        if let Some(map) = env_value.as_mapping() {
            for prefix in env_prefixes {
                for (key, _) in map.iter() {
                    if let Some(s) = key.as_str() {
                        if s.to_ascii_lowercase()
                            .starts_with(&prefix.to_ascii_lowercase())
                        {
                            if s.to_ascii_uppercase().ends_with("_DB")
                                || s.to_ascii_uppercase().ends_with("_DATABASE")
                            {
                                score += ENV_DB_SCORE;
                            } else {
                                score += ENV_CREDENTIAL_SCORE;
                            }
                        }
                    }
                }
            }
        } else if let Some(seq) = env_value.as_sequence() {
            for item in seq {
                if let Some(s) = item.as_str() {
                    for prefix in env_prefixes {
                        if s.to_ascii_lowercase()
                            .starts_with(&prefix.to_ascii_lowercase())
                        {
                            if s.to_ascii_uppercase().contains("_DB=")
                                || s.to_ascii_uppercase().contains("_DATABASE=")
                            {
                                score += ENV_DB_SCORE;
                            } else {
                                score += ENV_CREDENTIAL_SCORE;
                            }
                        }
                    }
                }
            }
        }
    }

    score
}

/// Detect which database service is in a docker-compose folder

pub fn find_db_service(folder_path: &PathBuf) -> std::io::Result<(DbType, String)> {
    let orchestrator_path = folder_path.join("docker-compose.yml");
    let docker_compose_text = fs::read_to_string(orchestrator_path)?;
    let docker_data = serializer_docker(docker_compose_text)?;

    let mut scores: HashMap<String, HashMap<DbType, i32>> = HashMap::new();

    for (name, svc) in &docker_data.services {
        let name_matched = NAME_LIST
            .iter()
            .any(|alias| name.eq_ignore_ascii_case(alias));
        let name_bonus = if name_matched { NAME_SCORE } else { 0 };

        let mut db_scores: HashMap<DbType, i32> = HashMap::new();
        db_scores.insert(DbType::Postgres, score_service_for_db(svc, PG_IMAGE_LIST, &[PG_PORT], &["POSTGRES_"]) + name_bonus);
        db_scores.insert(DbType::Mysql, score_service_for_db(svc, MYSQL_IMAGE_LIST, &[MYSQL_PORT, MYSQL_ALT_PORT], &["MYSQL_"]) + name_bonus);
        db_scores.insert(DbType::Mariadb, score_service_for_db(svc, MARIADB_IMAGE_LIST, &[MYSQL_PORT, MYSQL_ALT_PORT], &["MYSQL_", "MARIADB_"]) + name_bonus);
        db_scores.insert(DbType::Mongo, score_service_for_db(svc, MONGO_IMAGE_LIST, &[MONGO_PORT], &["MONGO_"]) + name_bonus);
        db_scores.insert(DbType::Sqlite, score_service_for_db(svc, SQLITE_IMAGE_LIST, &[], &[]) + name_bonus);

        scores.insert(name.clone(), db_scores);
    }

    // Find the best (service_name, DbType) combination
    let mut max_score = 0;
    let mut winner_service: Option<String> = None;
    let mut winner_db_type: Option<DbType> = None;

    for (svc_name, db_scores) in &scores {
        for (db_type, score) in db_scores {
            if *score > max_score {
                max_score = *score;
                winner_service = Some(svc_name.clone());
                winner_db_type = Some(db_type.clone());
            }
        }
    }

    match (winner_service, winner_db_type) {
        (Some(svc), Some(db_type)) if max_score >= MIN_DB_SCORE => Ok((db_type, svc)),
        _ => Err(Error::new(
            ErrorKind::NotFound,
            "No se encontró un servicio de base de datos",
        )),
    }
}

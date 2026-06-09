use std::collections::HashMap;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

use crate::compose::serializer_docker;
use crate::types::{DbType, GenericCredentials};

// ── Per-DB heuristic data ────────────────────────────────────────────────

/// Service name patterns that suggest a DB service (all DBs share these).
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
/// Order: Postgres-specific images come first, then MySQL, MariaDB, Mongo, SQLite.
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

/// Image score: 30 points for exact image match.
const IMAGE_SCORE: i32 = 30;
/// Name match score: 10 points for service name matching DB patterns.
const NAME_SCORE: i32 = 10;
/// Port match score: 25 points for standard DB port.
const PORT_SCORE: i32 = 25;
/// Minimum score to consider a service as a database.
/// A service with only the no-depends bonus (20 points) is NOT a DB.
const MIN_DB_SCORE: i32 = 30;

/// Env var score: 15-20 points per detected credential variable.
const ENV_CREDENTIAL_SCORE: i32 = 15;
const ENV_DB_SCORE: i32 = 20;
/// No-depends-on bonus: 20 points (DBs are typically leaf services).
const NO_DEPENDS_BONUS: i32 = 20;

/// Per-DB port defaults
const PG_PORT: &str = "5432";
const MYSQL_PORT: &str = "3306";
const MYSQL_ALT_PORT: &str = "3307";
const MONGO_PORT: &str = "27017";

/// Score a single service against one DB type.
fn score_service_for_db(
    svc: &crate::compose::Service,
    _db_type: DbType,
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

/// Extract a YAML env value by key (case-insensitive prefix). Returns `None` if not found.
fn yaml_env_get(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
    for (k, v) in map.iter() {
        if let Some(s) = k.as_str() {
            if s.eq_ignore_ascii_case(key) {
                return v.as_str().map(|s| s.to_string());
            }
        }
    }
    None
}

/// Extract an env var from a sequence of "KEY=VALUE" strings.
fn seq_env_get(seq: &[serde_yaml::Value], prefix: &str) -> Option<String> {
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

/// Extract string env var from either mapping or sequence.
fn extract_env(svc: &crate::compose::Service, key: &str) -> Option<String> {
    let env_val = svc.environment.as_ref()?;
    if let Some(map) = env_val.as_mapping() {
        yaml_env_get(map, key)
    } else if let Some(seq) = env_val.as_sequence() {
        seq_env_get(seq, key)
    } else {
        None
    }
}

pub fn find_db_service(folder_path: &PathBuf) -> std::io::Result<GenericCredentials> {
    let orchestrator_path = folder_path.join("docker-compose.yml");
    let docker_compose_text = fs::read_to_string(orchestrator_path)?;

    let docker_data = serializer_docker(docker_compose_text)?;

    // Per-service, per-DB-type scores: HashMap<service_name, HashMap<DbType, i32>>
    let mut scores: HashMap<String, HashMap<DbType, i32>> = HashMap::new();

    for (name, svc) in &docker_data.services {
        let mut db_scores: HashMap<DbType, i32> = HashMap::new();

        // Base: name match
        let name_matched = NAME_LIST
            .iter()
            .any(|alias| name.eq_ignore_ascii_case(alias));

        // Score each DB type
        let pg_score = score_service_for_db(
            svc,
            DbType::Postgres,
            PG_IMAGE_LIST,
            &[PG_PORT],
            &["POSTGRES_"],
        );
        let mysql_score = score_service_for_db(
            svc,
            DbType::Mysql,
            MYSQL_IMAGE_LIST,
            &[MYSQL_PORT, MYSQL_ALT_PORT],
            &["MYSQL_"],
        );
        let mariadb_score = score_service_for_db(
            svc,
            DbType::Mariadb,
            MARIADB_IMAGE_LIST,
            &[MYSQL_PORT, MYSQL_ALT_PORT],
            &["MYSQL_", "MARIADB_"],
        );
        let mongo_score = score_service_for_db(
            svc,
            DbType::Mongo,
            MONGO_IMAGE_LIST,
            &[MONGO_PORT],
            &["MONGO_"],
        );
        let sqlite_score = score_service_for_db(svc, DbType::Sqlite, SQLITE_IMAGE_LIST, &[], &[]);

        // Add name match bonus
        let name_bonus = if name_matched { NAME_SCORE } else { 0 };

        db_scores.insert(DbType::Postgres, pg_score + name_bonus);
        db_scores.insert(DbType::Mysql, mysql_score + name_bonus);
        db_scores.insert(DbType::Mariadb, mariadb_score + name_bonus);
        db_scores.insert(DbType::Mongo, mongo_score + name_bonus);
        db_scores.insert(DbType::Sqlite, sqlite_score + name_bonus);

        scores.insert(name.clone(), db_scores);
    }

    // Find the best (service_name, DbType, score) combination.
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

    let service = match winner_service {
        Some(ref winner) if max_score >= MIN_DB_SCORE => docker_data.services.get(winner).unwrap(),
        _ => {
            return Err(Error::new(
                ErrorKind::NotFound,
                "No se encontró un servicio de base de datos",
            ));
        }
    };

    let db_type = winner_db_type.expect("winner_db_type must exist when winner_service exists");

    // Extract credentials based on db_type.
    let (default_port, default_user, default_password, default_database): (&str, &str, &str, &str) =
        match db_type {
            DbType::Postgres => (PG_PORT, "postgres", "postgres", "appdb"),
            DbType::Mysql | DbType::Mariadb => (MYSQL_PORT, "root", "root", "mysql"),
            DbType::Mongo => (MONGO_PORT, "admin", "admin", "admin"),
            DbType::Sqlite => ("", "", "", ""),
        };

    // Extract port (host port, not container port)
    let mut port = String::from(default_port);
    if let Some(ports_vec) = &service.ports {
        if let Some(first) = ports_vec.first() {
            let host_port = first.split('/').next().unwrap_or(first);
            let parts: Vec<&str> = host_port.split(':').collect();
            port = parts.first().unwrap_or(&default_port).to_string();
        }
    }

    // Extract env vars
    let (user_key, password_key, db_key): (&str, &str, &str) = match db_type {
        DbType::Postgres => ("POSTGRES_USER", "POSTGRES_PASSWORD", "POSTGRES_DB"),
        DbType::Mysql | DbType::Mariadb => ("MYSQL_USER", "MYSQL_PASSWORD", "MYSQL_DATABASE"),
        DbType::Mongo => (
            "MONGO_INITDB_ROOT_USERNAME",
            "MONGO_INITDB_ROOT_PASSWORD",
            "MONGO_INITDB_DATABASE",
        ),
        DbType::Sqlite => ("", "", ""),
    };

    let user = match db_type {
        DbType::Mysql | DbType::Mariadb => {
            // MySQL: prefer MYSQL_USER, fallback to root
            extract_env(service, user_key)
                .or_else(|| {
                    // For MySQL root fallback, MYSQL_ROOT_PASSWORD implies user=root
                    if extract_env(service, "MYSQL_ROOT_PASSWORD").is_some()
                        || extract_env(service, "MARIADB_ROOT_PASSWORD").is_some()
                    {
                        Some("root".to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| String::from(default_user))
        }
        DbType::Mongo => {
            extract_env(service, user_key).unwrap_or_else(|| String::from(default_user))
        }
        DbType::Postgres => {
            extract_env(service, user_key).unwrap_or_else(|| String::from(default_user))
        }
        DbType::Sqlite => String::new(),
    };

    let password = match db_type {
        DbType::Mysql | DbType::Mariadb => extract_env(service, password_key)
            .or_else(|| extract_env(service, "MYSQL_ROOT_PASSWORD"))
            .or_else(|| extract_env(service, "MARIADB_ROOT_PASSWORD"))
            .unwrap_or_else(|| String::from(default_password)),
        DbType::Mongo => {
            extract_env(service, password_key).unwrap_or_else(|| String::from(default_password))
        }
        DbType::Postgres => {
            extract_env(service, password_key).unwrap_or_else(|| String::from(default_password))
        }
        DbType::Sqlite => String::new(),
    };

    let database = match db_type {
        DbType::Sqlite => String::new(),
        _ => extract_env(service, db_key).unwrap_or_else(|| String::from(default_database)),
    };

    let host = match db_type {
        DbType::Sqlite => String::new(), // file-based, no host
        _ => String::from("localhost"),
    };

    Ok(GenericCredentials {
        db_type,
        host,
        port,
        user,
        password,
        database,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DbType;
    use std::fs;
    use std::path::PathBuf;

    fn write_compose(dir: &PathBuf, content: &str) {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join("docker-compose.yml");
        fs::write(&path, content).unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dl_test_{name}"));
        // Best-effort cleanup from previous runs
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    // ── PostgreSQL detection (regression) ──────────────────────────────

    #[test]
    fn test_postgres_detected() {
        let dir = test_dir("pg_detected");
        write_compose(
            &dir,
            r#"
services:
  db:
    image: postgres:15
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: pguser
      POSTGRES_PASSWORD: pgpass
      POSTGRES_DB: pgdb
"#,
        );
        let result = find_db_service(&dir).unwrap();
        assert_eq!(
            result.db_type,
            DbType::Postgres,
            "PostgreSQL service should be detected as Postgres"
        );
        assert_eq!(result.port, "5432");
        assert_eq!(result.user, "pguser");
        assert_eq!(result.password, "pgpass");
        assert_eq!(result.database, "pgdb");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_postgres_with_depends_on_still_detected() {
        let dir = test_dir("pg_depends");
        write_compose(
            &dir,
            r#"
services:
  db:
    image: postgres:16
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: pg
      POSTGRES_PASSWORD: secret
    depends_on:
      - some-service
"#,
        );
        let result = find_db_service(&dir).unwrap();
        assert_eq!(
            result.db_type,
            DbType::Postgres,
            "PostgreSQL with depends_on should still be detected via image + port + env"
        );
        // No-depends bonus lost but image (30) + port (25) + env (15+15) = 85 > 30
        let _ = fs::remove_dir_all(&dir);
    }

    // ── MySQL detection ────────────────────────────────────────────────

    #[test]
    fn test_mysql_detected() {
        let dir = test_dir("mysql_detected");
        write_compose(
            &dir,
            r#"
services:
  mysql:
    image: mysql:8
    ports:
      - "3306:3306"
    environment:
      MYSQL_ROOT_PASSWORD: rootpass
      MYSQL_DATABASE: mydb
      MYSQL_USER: myuser
      MYSQL_PASSWORD: mypass
"#,
        );
        let result = find_db_service(&dir).unwrap();
        assert_eq!(
            result.db_type,
            DbType::Mysql,
            "MySQL service should be detected as Mysql"
        );
        assert_eq!(result.port, "3306");
        assert_eq!(result.user, "myuser");
        assert_eq!(result.password, "mypass");
        assert_eq!(result.database, "mydb");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_mysql_minimal_credentials() {
        let dir = test_dir("mysql_minimal");
        write_compose(
            &dir,
            r#"
services:
  database:
    image: mysql:8
    ports:
      - "3306:3306"
    environment:
      MYSQL_ROOT_PASSWORD: secret
"#,
        );
        let result = find_db_service(&dir).unwrap();
        assert_eq!(result.db_type, DbType::Mysql);
        // MySQL defaults when env vars missing
        assert_eq!(result.user, "root");
        assert_eq!(result.password, "secret");
        assert_eq!(result.database, "mysql");
        let _ = fs::remove_dir_all(&dir);
    }

    // ── MariaDB detection ──────────────────────────────────────────────

    #[test]
    fn test_mariadb_detected_by_image() {
        let dir = test_dir("mariadb_img");
        write_compose(
            &dir,
            r#"
services:
  db:
    image: mariadb:11
    ports:
      - "3306:3306"
    environment:
      MYSQL_ROOT_PASSWORD: rootpass
      MYSQL_DATABASE: appdb
"#,
        );
        let result = find_db_service(&dir).unwrap();
        assert_eq!(
            result.db_type,
            DbType::Mariadb,
            "MariaDB image should be detected as Mariadb, not Mysql"
        );
        assert_eq!(result.port, "3306");
        let _ = fs::remove_dir_all(&dir);
    }

    // ── SQLite detection ───────────────────────────────────────────────

    #[test]
    fn test_sqlite_detected() {
        let dir = test_dir("sqlite_detected");
        write_compose(
            &dir,
            r#"
services:
  sqlite:
    image: keinos/sqlite3
    volumes:
      - ./data:/data
"#,
        );
        let result = find_db_service(&dir).unwrap();
        assert_eq!(
            result.db_type,
            DbType::Sqlite,
            "SQLite service should be detected as Sqlite"
        );
        // SQLite is file-based, no container IP needed
        assert_eq!(result.host, "");
        assert_eq!(result.port, "");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_sqlite_alias_image() {
        let dir = test_dir("sqlite_alias");
        write_compose(
            &dir,
            r#"
services:
  sqlite3:
    image: sqlite3
"#,
        );
        let result = find_db_service(&dir).unwrap();
        assert_eq!(result.db_type, DbType::Sqlite);
        let _ = fs::remove_dir_all(&dir);
    }

    // ── MongoDB detection ──────────────────────────────────────────────

    #[test]
    fn test_mongodb_detected() {
        let dir = test_dir("mongo_detected");
        write_compose(
            &dir,
            r#"
services:
  mongo:
    image: mongo:7
    ports:
      - "27017:27017"
    environment:
      MONGO_INITDB_ROOT_USERNAME: admin
      MONGO_INITDB_ROOT_PASSWORD: adminpass
      MONGO_INITDB_DATABASE: appdb
"#,
        );
        let result = find_db_service(&dir).unwrap();
        assert_eq!(
            result.db_type,
            DbType::Mongo,
            "MongoDB service should be detected as Mongo"
        );
        assert_eq!(result.port, "27017");
        assert_eq!(result.user, "admin");
        assert_eq!(result.password, "adminpass");
        assert_eq!(result.database, "appdb");
        let _ = fs::remove_dir_all(&dir);
    }

    // ── Negative / edge cases ──────────────────────────────────────────

    #[test]
    fn test_no_db_found() {
        let dir = test_dir("no_db");
        write_compose(
            &dir,
            r#"
services:
  web:
    image: nginx:alpine
    ports:
      - "80:80"
"#,
        );
        let result = find_db_service(&dir);
        assert!(result.is_err(), "Should not find any DB service");
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::NotFound);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ambiguous_multi_db() {
        let dir = test_dir("ambiguous_multi");
        write_compose(
            &dir,
            r#"
services:
  postgres:
    image: postgres:15
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: pguser
      POSTGRES_PASSWORD: pgpass
      POSTGRES_DB: pgdb
  mysql:
    image: mysql:8
    ports:
      - "3306:3306"
    environment:
      MYSQL_ROOT_PASSWORD: rootpass
      MYSQL_DATABASE: mydb
"#,
        );
        let result = find_db_service(&dir).unwrap();
        // Either PG or MySQL wins — the function must return ONE.
        // Both are valid winners; verify the type matches the winner.
        assert!(
            result.db_type == DbType::Postgres || result.db_type == DbType::Mysql,
            "Multi-DB compose must return one of the DB types, got {:?}",
            result.db_type
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

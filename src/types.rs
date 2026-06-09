use serde::{Deserialize, Serialize};

/// Supported database engine types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DbType {
    Postgres,
    Mysql,
    Mariadb,
    Sqlite,
    Mongo,
}

/// Generic database credentials, replacing PG-specific `DbData`.
/// The `db_type` discriminator drives dispatch in `db/mod.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericCredentials {
    pub db_type: DbType,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
}

/// Legacy PG-specific credentials. Kept as a compatibility alias.
/// Convert to `GenericCredentials` via `From<DbData>`.
#[derive(Debug, Clone)]
pub struct DbData {
    pub port: String,
    pub postgres_user: String,
    pub postgres_password: String,
    pub postgres_db: String,
}

impl From<DbData> for GenericCredentials {
    fn from(d: DbData) -> Self {
        GenericCredentials {
            db_type: DbType::Postgres,
            host: "localhost".to_string(),
            port: d.port,
            user: d.postgres_user,
            password: d.postgres_password,
            database: d.postgres_db,
        }
    }
}

#[derive(Debug)]
pub struct TablaInfo {
    pub nombre: String,
    pub columnas: Vec<ColumnaInfo>,
}

#[derive(Debug)]
pub struct ColumnaInfo {
    pub nombre: String,
    pub tipo: String,
    pub nullable: String,
    pub default: Option<String>,
}

// SQLite schema types

pub struct SQLiteSchema {
    pub tables: Vec<SQLiteTable>,
}

pub struct SQLiteTable {
    pub name: String,
    pub columns: Vec<SQLiteColumn>,
}

pub struct SQLiteColumn {
    pub name: String,
    pub sqlite_type: String, 
    pub nullable: bool,
    pub default: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbtype_variants_exist() {
        // Verify all five variants are constructible
        let _pg = DbType::Postgres;
        let _mysql = DbType::Mysql;
        let _mariadb = DbType::Mariadb;
        let _sqlite = DbType::Sqlite;
        let _mongo = DbType::Mongo;
    }

    #[test]
    fn test_dbtype_equality() {
        assert_eq!(DbType::Postgres, DbType::Postgres);
        assert_ne!(DbType::Postgres, DbType::Mysql);
        assert_ne!(DbType::Mysql, DbType::Mariadb);
    }

    #[test]
    fn test_generic_credentials_construction() {
        let creds = GenericCredentials {
            db_type: DbType::Postgres,
            host: "localhost".to_string(),
            port: "5432".to_string(),
            user: "postgres".to_string(),
            password: "secret".to_string(),
            database: "appdb".to_string(),
        };
        assert_eq!(creds.db_type, DbType::Postgres);
        assert_eq!(creds.host, "localhost");
        assert_eq!(creds.port, "5432");
        assert_eq!(creds.user, "postgres");
        assert_eq!(creds.password, "secret");
        assert_eq!(creds.database, "appdb");
    }

    #[test]
    fn test_dbdata_to_generic_conversion() {
        let old = DbData {
            port: "5432".to_string(),
            postgres_user: "pguser".to_string(),
            postgres_password: "pgpass".to_string(),
            postgres_db: "pgdb".to_string(),
        };
        let creds: GenericCredentials = old.into();
        assert_eq!(creds.db_type, DbType::Postgres);
        assert_eq!(creds.host, "localhost");
        assert_eq!(creds.port, "5432");
        assert_eq!(creds.user, "pguser");
        assert_eq!(creds.password, "pgpass");
        assert_eq!(creds.database, "pgdb");
    }

    #[test]
    fn test_generic_credentials_mysql() {
        let creds = GenericCredentials {
            db_type: DbType::Mysql,
            host: "10.0.0.5".to_string(),
            port: "3306".to_string(),
            user: "root".to_string(),
            password: "rootpass".to_string(),
            database: "mydb".to_string(),
        };
        assert_eq!(creds.db_type, DbType::Mysql);
        assert_eq!(creds.port, "3306");
    }

    #[test]
    fn test_dbtype_clone() {
        let a = DbType::Sqlite;
        let b = a.clone();
        assert_eq!(a, b);
    }
}

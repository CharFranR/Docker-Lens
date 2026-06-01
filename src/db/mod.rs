// Multi-DB dispatch module.
// Routes `list_tables`, `make_query`, `export_csv` to engine-specific adapters
// based on `GenericCredentials.db_type`.
pub mod postgres;

use std::io::{Error, ErrorKind};
use crate::types::{DbType, GenericCredentials};

/// List all tables/collections for the given credentials.
pub fn list_tables(creds: &GenericCredentials) -> std::io::Result<String> {
    match creds.db_type {
        DbType::Postgres => postgres::list_tables(creds),
        _ => Err(Error::new(
            ErrorKind::Unsupported,
            format!("list_tables: unsupported db_type {:?}", creds.db_type),
        )),
    }
}

/// Execute an arbitrary query against the target database.
pub fn make_query(creds: &GenericCredentials, query: &str) -> std::io::Result<String> {
    match creds.db_type {
        DbType::Postgres => postgres::make_query(creds, query),
        _ => Err(Error::new(
            ErrorKind::Unsupported,
            format!("make_query: unsupported db_type {:?}", creds.db_type),
        )),
    }
}

/// Export a table to CSV at the given file path.
pub fn export_csv(creds: &GenericCredentials, table: &str, path: &str) -> std::io::Result<()> {
    match creds.db_type {
        DbType::Postgres => postgres::export_csv(creds, table, path),
        _ => Err(Error::new(
            ErrorKind::Unsupported,
            format!("export_csv: unsupported db_type {:?}", creds.db_type),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_dispatch_list_tables_postgres_routes_correctly() {
        let creds = pg_creds();
        // Routes to real postgres adapter — succeeds if psql installed (PG may be down).
        // Must NOT return Unsupported (proves dispatch matched Postgres arm).
        let result = list_tables(&creds);
        match result {
            Ok(_) => {} // psql installed, returned stdout (possibly empty if PG down)
            Err(e) => assert_ne!(e.kind(), ErrorKind::Unsupported,
                "Postgres should NOT be Unsupported — dispatch routing failed"),
        }
    }

    #[test]
    fn test_dispatch_list_tables_unsupported() {
        let creds = mysql_creds();
        let result = list_tables(&creds);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Unsupported);
        assert!(err.to_string().contains("unsupported"));
    }

    #[test]
    fn test_dispatch_make_query_postgres_routes_correctly() {
        let creds = pg_creds();
        let result = make_query(&creds, "SELECT 1");
        match result {
            Ok(_) => {} // PG is up and running
            Err(e) => assert_ne!(e.kind(), ErrorKind::Unsupported,
                "Postgres should NOT be Unsupported — dispatch routing failed"),
        }
    }

    #[test]
    fn test_dispatch_make_query_unsupported() {
        let creds = mysql_creds();
        let result = make_query(&creds, "SELECT 1");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), ErrorKind::Unsupported);
    }

    #[test]
    fn test_dispatch_export_csv_postgres_routes_correctly() {
        let creds = pg_creds();
        let result = export_csv(&creds, "users", "/tmp/test.csv");
        match result {
            Ok(_) => {} // PG is up and running
            Err(e) => assert_ne!(e.kind(), ErrorKind::Unsupported,
                "Postgres should NOT be Unsupported — dispatch routing failed"),
        }
    }

    #[test]
    fn test_dispatch_export_csv_unsupported() {
        let creds = mysql_creds();
        let result = export_csv(&creds, "users", "/tmp/test.csv");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), ErrorKind::Unsupported);
    }

    #[test]
    fn test_dispatch_all_unsupported_types() {
        for db_type in [DbType::Mysql, DbType::Mariadb, DbType::Sqlite, DbType::Mongo] {
            let creds = GenericCredentials {
                db_type,
                host: "localhost".into(),
                port: "0".into(),
                user: "u".into(),
                password: "p".into(),
                database: "d".into(),
            };
            let r = list_tables(&creds);
            assert!(r.is_err(), "list_tables should fail for {:?}", creds.db_type);
            let r = make_query(&creds, "x");
            assert!(r.is_err(), "make_query should fail for {:?}", creds.db_type);
            let r = export_csv(&creds, "x", "/tmp/x");
            assert!(r.is_err(), "export_csv should fail for {:?}", creds.db_type);
        }
    }
}

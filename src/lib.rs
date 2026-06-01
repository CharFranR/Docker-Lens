use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::{Path, PathBuf};

mod compose;
mod db;
mod heuristic;
mod scanner;
mod types;

use crate::db::export_to_sqlite;
use crate::heuristic::find_db_service;
use crate::scanner::find_container_orchestrator;
use crate::types::{DbType, GenericCredentials};

// Wrappers marca chapi, esta como peluda la libreria Py03

/// Extract a required string key from a PyDict.
fn extract_str(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<String> {
    dict.get_item(key)?
        .ok_or_else(|| PyRuntimeError::new_err(format!("Missing key: {key}")))?
        .extract()
}

/// Build GenericCredentials from a Python dict.
fn dict_to_creds(dict: &Bound<'_, PyDict>) -> PyResult<GenericCredentials> {
    let db_type_str: String = extract_str(dict, "db_type")?;
    let db_type = match db_type_str.as_str() {
        "postgres" => types::DbType::Postgres,
        "mysql" => types::DbType::Mysql,
        "mariadb" => types::DbType::Mariadb,
        "sqlite" => types::DbType::Sqlite,
        "mongo" => types::DbType::Mongo,
        other => return Err(PyRuntimeError::new_err(format!("Unknown db_type: {other}"))),
    };
    Ok(GenericCredentials {
        db_type,
        host: extract_str(dict, "host")?,
        port: extract_str(dict, "port")?,
        user: extract_str(dict, "user")?,
        password: extract_str(dict, "password")?,
        database: extract_str(dict, "database")?,
    })
}

#[pyfunction]
fn find_orchestrator_py(file_path: String) -> PyResult<String> {
    let path = Path::new(&file_path);
    match find_container_orchestrator(path) {
        Ok(p) => Ok(p.to_string_lossy().to_string()),
        Err(e) => Err(PyRuntimeError::new_err(format!("{}", e))),
    }
}

#[pyfunction]
fn find_db_py(py: Python<'_>, file_path: String) -> PyResult<Bound<'_, PyDict>> {
    let path = PathBuf::from(&file_path);
    let creds = find_db_service(&path).map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

    let db_type_str = match creds.db_type {
        DbType::Postgres => "postgres",
        DbType::Mysql => "mysql",
        DbType::Mariadb => "mariadb",
        DbType::Sqlite => "sqlite",
        DbType::Mongo => "mongo",
    };

    let dict = PyDict::new(py);
    dict.set_item("db_type", db_type_str)?;
    dict.set_item("host", &creds.host)?;
    dict.set_item("port", &creds.port)?;
    dict.set_item("user", &creds.user)?;
    dict.set_item("password", &creds.password)?;
    dict.set_item("database", &creds.database)?;
    Ok(dict)
}

#[pyfunction]
fn list_tables_py(credenciales: &Bound<'_, PyDict>) -> PyResult<String> {
    let creds = dict_to_creds(credenciales)?;
    crate::db::list_tables(&creds).map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
}

#[pyfunction]
fn make_query_py(credenciales: &Bound<'_, PyDict>, query: String) -> PyResult<String> {
    let creds = dict_to_creds(credenciales)?;
    crate::db::make_query(&creds, &query).map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
}

#[pyfunction]
fn export_csv_py(
    credenciales: &Bound<'_, PyDict>,
    table_name: String,
    file_path: String,
) -> PyResult<()> {
    let creds = dict_to_creds(credenciales)?;
    crate::db::export_csv(&creds, &table_name, &file_path)
        .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
}

#[pyfunction]
fn export_to_sqlite_py(credenciales: &Bound<'_, PyDict>, sqlite_path: String) -> PyResult<()> {
    let creds = dict_to_creds(credenciales)?;
    export_to_sqlite(&creds, &sqlite_path).map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
}

#[pyfunction]
fn inspect_schema_py(py: Python<'_>, credenciales: &Bound<'_, PyDict>) -> PyResult<Vec<Py<PyAny>>> {
    let creds = dict_to_creds(credenciales)?;
    let tables = crate::db::inspect_schema(&creds)
        .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;

    let result: Vec<Py<PyAny>> = tables
        .iter()
        .map(|t| {
            let dict = PyDict::new(py);
            dict.set_item("name", &t.nombre).unwrap();
            let columns: Vec<Py<PyAny>> = t
                .columnas
                .iter()
                .map(|c| {
                    let col_dict = PyDict::new(py);
                    col_dict.set_item("name", &c.nombre).unwrap();
                    col_dict.set_item("type", &c.tipo).unwrap();
                    col_dict.set_item("nullable", &c.nullable).unwrap();
                    col_dict.set_item("default", &c.default).unwrap();
                    col_dict.into_any().unbind()
                })
                .collect();
            dict.set_item("columns", columns).unwrap();
            dict.into_any().unbind()
        })
        .collect();

    Ok(result)
}

#[pyfunction]
fn get_container_ip_py(credenciales: &Bound<'_, PyDict>, service_name: String) -> PyResult<Option<String>> {
    let creds = dict_to_creds(credenciales)?;
    Ok(crate::db::get_container_ip(&creds, &service_name))
}

#[pymodule]
fn docker_lens(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(find_orchestrator_py, m)?)?;
    m.add_function(wrap_pyfunction!(find_db_py, m)?)?;
    m.add_function(wrap_pyfunction!(list_tables_py, m)?)?;
    m.add_function(wrap_pyfunction!(make_query_py, m)?)?;
    m.add_function(wrap_pyfunction!(export_csv_py, m)?)?;
    m.add_function(wrap_pyfunction!(export_to_sqlite_py, m)?)?;
    m.add_function(wrap_pyfunction!(inspect_schema_py, m)?)?;
    m.add_function(wrap_pyfunction!(get_container_ip_py, m)?)?;
    Ok(())
}

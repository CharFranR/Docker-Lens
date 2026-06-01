use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyDict;
use std::path::{Path, PathBuf};

mod scanner;
mod heuristic;
mod compose;
mod psql;
mod types;

use crate::scanner::find_container_orchestrator;
use crate::heuristic::find_db_service;
use crate::psql::{list_tables, make_query, export_csv, export_to_sqlite};


// Wrappers marca chapi, esta como peluda la libreria Py03

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
    let data = find_db_service(&path)
        .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))?;
    let dict = PyDict::new(py);
    dict.set_item("port", &data.port)?;
    dict.set_item("postgres_user", &data.postgres_user)?;
    dict.set_item("postgres_password", &data.postgres_password)?;
    dict.set_item("postgres_db", &data.postgres_db)?;
    Ok(dict)
}

#[pyfunction]
fn list_tables_py(credenciales: &Bound<'_, PyDict>) -> PyResult<String> {
    let port = credenciales.get_item("port")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: port"))?.extract()?;
    let user = credenciales.get_item("postgres_user")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: port"))?.extract()?;
    let password = credenciales.get_item("postgres_password")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: port"))?.extract()?;
    let db = credenciales.get_item("postgres_db")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: port"))?.extract()?;
    let credentials = types::DbData {
        port,
        postgres_user: user,
        postgres_password: password,
        postgres_db: db,
    };
    list_tables(&credentials)
        .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
}


#[pyfunction]
fn make_query_py(credenciales: &Bound<'_, PyDict>, query: String) -> PyResult<String> {
    let port = credenciales.get_item("port")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: port"))?.extract()?;
    let user = credenciales.get_item("postgres_user")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: postgres_user"))?.extract()?;
    let password = credenciales.get_item("postgres_password")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: postgres_password"))?.extract()?;
    let db = credenciales.get_item("postgres_db")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: postgres_db"))?.extract()?;
    let credentials = types::DbData {
        port,
        postgres_user: user,
        postgres_password: password,
        postgres_db: db,
    };
    make_query(&credentials, &query)
        .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
}

#[pyfunction]
fn export_csv_py(credenciales: &Bound<'_, PyDict>, table_name: String, file_path: String) -> PyResult<()> {
    let port = credenciales.get_item("port")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: port"))?.extract()?;
    let user = credenciales.get_item("postgres_user")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: postgres_user"))?.extract()?;
    let password = credenciales.get_item("postgres_password")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: postgres_password"))?.extract()?;
    let db = credenciales.get_item("postgres_db")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: postgres_db"))?.extract()?;
    let credentials = types::DbData {
        port,
        postgres_user: user,
        postgres_password: password,
        postgres_db: db,
    };
    export_csv(&credentials, &table_name, &file_path)
        .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
}


#[pyfunction]
fn export_to_sqlite_py(credenciales: &Bound<'_, PyDict>, sqlite_path: String) -> PyResult<()> {
    let port = credenciales.get_item("port")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: port"))?.extract()?;
    let user = credenciales.get_item("postgres_user")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: postgres_user"))?.extract()?;
    let password = credenciales.get_item("postgres_password")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: postgres_password"))?.extract()?;
    let db = credenciales.get_item("postgres_db")?.ok_or_else(|| PyRuntimeError::new_err("Missing key: postgres_db"))?.extract()?;
    let credentials = types::DbData {
        port,
        postgres_user: user,
        postgres_password: password,
        postgres_db: db,
    };
    export_to_sqlite(&credentials, &sqlite_path)
        .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
}

#[pymodule]
fn docker_lens(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(find_orchestrator_py, m)?)?;
    m.add_function(wrap_pyfunction!(find_db_py, m)?)?;
    m.add_function(wrap_pyfunction!(list_tables_py, m)?)?;
    m.add_function(wrap_pyfunction!(make_query_py, m)?)?;
    m.add_function(wrap_pyfunction!(export_csv_py, m)?)?;
    m.add_function(wrap_pyfunction!(export_to_sqlite_py, m)?)?;
    Ok(())
}

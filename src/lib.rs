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
use crate::psql::list_tables;

const TEST_PATH: &str = "/home/frandev/Documentos/Proyecto-Asignatura-Web";

pub fn run (folder_path:&str) {

    let init_folder_path: &Path = Path::new(&folder_path);

    let docker_path = find_container_orchestrator(init_folder_path);


    let db_service_data = match docker_path {
        Ok(value) => {
            find_db_service(&value)
        }

        Err(err) => {
            println!("err");
            return;
        }
    };

    let db_tables = match db_service_data  {
        Ok(value) => {
            list_tables(&value)
        }

        Err(err) => {
            println!("Falla");
            return;
        }  
    };

    match db_tables {
        Ok(tables) => println!("{}", tables),
        Err(e) => eprintln!("Error al listar tablas: {}", e),
    }

}

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
fn list_tables_py(user: String, password: String, db: String, port: String) -> PyResult<String> {
    let credentials = types::DbData {
        port,
        postgres_user: user,
        postgres_password: password,
        postgres_db: db,
    };
    list_tables(&credentials)
        .map_err(|e| PyRuntimeError::new_err(format!("{}", e)))
}


#[pymodule]
fn docker_lens(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(find_orchestrator_py, m)?)?;
    m.add_function(wrap_pyfunction!(find_db_py, m)?)?;
    m.add_function(wrap_pyfunction!(list_tables_py, m)?)?;
    Ok(())
}


fn main() {
    run(TEST_PATH);
}
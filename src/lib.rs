use pyo3::prelude::*;
use std::path::PathBuf;

mod docker;

// Re-export para usar en Python
pub use docker::{DB_Data, find_db_service, find_container_orchestrator, list_tables};

/// A Python module implemented in Rust.
#[pymodule]
mod docker_lens {
    use pyo3::prelude::*;
    use std::path::PathBuf;
    use crate::docker::{DB_Data, find_db_service, find_container_orchestrator, list_tables};

    #[pyfunction]
    fn find_orchestrator(file_path: String) -> PyResult<String> {
        let path = PathBuf::from(file_path);
        match find_container_orchestrator(&path) {
            Ok(path) => Ok(path.to_string_lossy().to_string()),
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!("IO Error: {}", e))),
        }
    }

    #[pyfunction]
    fn find_db(py: Python<'_>, file_path: String) -> PyResult<PyObject> {
        let path = PathBuf::from(file_path);
        
        match find_db_service(&path) {
            Ok(data) => {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("port", &data.port)?;
                dict.set_item("POSTGRES_USER", &data.POSTGRES_USER)?;
                dict.set_item("POSTGRES_PASSWORD", &data.POSTGRES_PASSWORD)?;
                dict.set_item("POSTGRES_DB", &data.POSTGRES_DB)?;
                Ok(dict.into())
            }
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!("IO Error: {}", e))),
        }
    }

    #[pyfunction]
    fn get_tables(user: String, password: String, db: String, port: String) -> PyResult<String> {
        let credentials = DB_Data {
            port,
            POSTGRES_USER: user,
            POSTGRES_PASSWORD: password,
            POSTGRES_DB: db,
        };
        
        // Capturar el output de println!
        let result = std::panic::catch_unwind(|| {
            list_tables(&credentials);
        });
        
        Ok("Tables listed (check terminal output)".to_string())
    }
    
    #[pyfunction]
    fn get_tables_return(user: String, password: String, db: String, port: String) -> PyResult<String> {
        use std::process::Command;
        
        let output = Command::new("psql")
            .args([
                "-hlocalhost",
                &format!("-p{}", port),
                &format!("-U{}", user),
                &format!("-d{}", db),
                "-c", "\\dt",
                "-w"
            ])
            .env("PGPASSWORD", &password)
            .output()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("psql not found: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                format!("psql failed: {}", stderr.trim())
            ));
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    #[pyfunction]
    fn execute_query(user: String, password: String, db: String, port: String, query: String) -> PyResult<String> {
        use std::process::Command;
        
        let output = Command::new("psql")
            .args([
                "-hlocalhost",
                &format!("-p{}", port),
                &format!("-U{}", user),
                &format!("-d{}", db),
                "-c", &query,
                "-w"
            ])
            .env("PGPASSWORD", &password)
            .output()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("psql not found: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                format!("Query failed: {}", stderr.trim())
            ));
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

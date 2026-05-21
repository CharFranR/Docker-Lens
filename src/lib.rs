use pyo3::prelude::*;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

mod scanner;
mod heuristic;
mod compose;
mod psql;
mod types;

use crate::scanner::find_container_orchestrator;
use crate::heuristic::find_db_service;
use crate::psql::list_tables;

const path: &str = "/home/frandev/Documentos/Proyecto-Asignatura-Web";

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


fn main() {
    eprintln!("DEBUG: run() called with path: {}", path);
    run(path);
}
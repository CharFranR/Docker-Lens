use std::string;
use std::path::{Path, PathBuf};
mod docker;

#[tokio::main]

async fn main () {

    println!("La prueba doctor\n");

    // Primeros nos conectamos a docker

    let client = match docker::connect_docker().await{
        Ok(c) => {
            println!("Conexión exitosa");
            c
        },
        
        Err(e) => {
            println!("No se ha podido encontrar un contenedor docker activo {}", e);
            return;
        } 
    };


    // docker::docker_version(&client).await;

    // docker::docker_images(&client).await;

    let file_path: String = String::from(r"/home/frandev/Documentos/Proyecto-Asignatura-Web/");

    // docker::list_files(&file_path).await;


    let orchestrator_path: PathBuf = match docker::find_container_orchestrator(Path::new(&file_path)){
        Ok (C) => {
            println!("Orquestador encontrado");
            println!("Ruta: {:?}", C);
            C
        },
        Err (e) => {
            println!("No ha sido posible encontrar el orquestador");
            return;
        }
    };

    // Leer el archivo y encontrar servicio de BD
    let credentials = match docker::find_db_service(&orchestrator_path) {
        Ok(service_name) => {
            println!("Servicio de BD encontrado: {:?}", service_name);
            service_name
            
        }
        Err(e) => {
            println!("Error al encontrar servicio de BD: {}", e);
            return;
        }
    };


    docker::list_tables(&credentials);

    println!("\nPrueba purrungueada");
}
use std::string;
use std::path::PathBuf;
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

    let file_path: String = String::from(r"");

    // docker::list_files(&file_path).await;


    let orchestrator_path: PathBuf = match docker::find_container_orchestrator(&file_path).await{
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

    
    
    
    println!("\nPrueba purrungueada");
}
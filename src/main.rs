use std::string;

mod docker;

#[tokio::main]

async fn main () {

    println!("La prueba doctor\n");

    // Primeros nos conectamos a docker

    let client = match docker::connect_docker().await{
        Ok(c) => {
            print!("Conexión exitosa");
            c
        },
        
        Err(e) => {
            print!("No se ha podido encontrar un contenedor docker activo {}", e);
            return;
        } 
    };


    // docker::docker_version(&client).await;

    // docker::docker_images(&client).await;

    let file_path: String = String::from(r"C:\Users\oscar\Documents\Biogestor");

    docker::list_files(&file_path).await;
    
    
    println!("\nPrueba purrungueada");
}
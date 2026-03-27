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


    docker::docker_version(&client).await;

    docker::docker_images(&client).await;


    println!("\nPrueba purrungueada");
}
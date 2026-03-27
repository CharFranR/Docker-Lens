mod docker;

#[tokio::main]

async fn main () {

    println!("La prueba doctor\n");

    docker::docker_version().await;

    docker::docker_images().await;


    println!("\nPrueba purrungueada");
}
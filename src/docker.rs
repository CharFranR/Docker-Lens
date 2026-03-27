

use bollard::Docker;
use bollard::errors::Error as DockerError;
use bollard::query_parameters::ListImagesOptionsBuilder;

pub async fn connect_docker () -> Result<Docker, DockerError> {
    let docker_client = Docker::connect_with_local_defaults()?;

    Ok(docker_client)
}

pub async fn docker_version (docker: &Docker){

    match docker.version().await{
        Ok (v) => println!("La version de docker es: {:?}", v),
        Err (e) => println!("Error a obtener la version de docker {}", e),
    }
}

pub async fn docker_images (docker: &Docker){

    let options = ListImagesOptionsBuilder::default()
        .all(true)
        .build();

    let images = &docker.list_images(Some(options)).await.unwrap();

    for image in images {
        println!("-> {:?}", image);
    }
}
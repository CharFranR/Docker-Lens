use bollard::Docker;
use bollard::errors::Error as DockerError;
use bollard::query_parameters::ListImagesOptionsBuilder;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};

#[derive(Debug, Deserialize)]
pub struct DockerCompose {
    pub services: HashMap<String, Service>,
}

#[derive(Debug, Deserialize)]
pub struct Service {
    pub container_name: Option<String>,
    pub image: Option<String>,
    pub ports: Option<Vec<String>>,
    pub volumes: Option<Vec<String>>,
    pub environment: Option<serde_yaml::Value>,
    pub depends_on: Option<serde_yaml::Value>, // Usamos Value porque puede ser lista o mapa
}

pub async fn connect_docker() -> Result<Docker, DockerError> {
    let docker_client = Docker::connect_with_local_defaults()?;

    Ok(docker_client)
}

pub async fn docker_version(docker: &Docker) {
    match docker.version().await {
        Ok(v) => println!("La version de docker es: {:?}", v),
        Err(e) => println!("Error a obtener la version de docker {}", e),
    }
}

pub async fn docker_images(docker: &Docker) {
    let options = ListImagesOptionsBuilder::default().all(true).build();

    let images = &docker.list_images(Some(options)).await.unwrap();

    for image in images {
        println!("-> {:?}", image);
    }
}

pub fn serializer_docker(docker_compose_text: String) -> Result<DockerCompose, Error> {
    let compose_data: DockerCompose = serde_yaml::from_str(&docker_compose_text)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Error en el YAML: {}", e)))?;

    Ok(compose_data)
}

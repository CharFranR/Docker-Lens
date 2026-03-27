use bollard::Docker;
use bollard::errors::Error as DockerError;
use bollard::query_parameters::ListImagesOptionsBuilder;
use walkdir::WalkDir;
use std::path::PathBuf;
use std::io::{Error, ErrorKind};
use std::fs;

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

pub async  fn list_files (file_path: &String) -> std::io::Result<()>{

    for entry in fs::read_dir(file_path)?{
        let entry = entry?;
        let path = entry.path();

        if path.is_file(){
            println!("Archivo: {:?}", path.file_name().unwrap());
        }
    }
    Ok(())
}

pub async fn find_container_orchestrator (file_path: &String) -> std::io::Result<(PathBuf)>{

    let target = "docker-compose.yml";

    for entry in WalkDir::new(file_path)
    .into_iter()
    .filter_map(|e| e.ok()) {
        if entry.file_name() == target {
            println!("Archivo encontrado en la ruta: {:?}", entry.path());
            return Ok(entry.path().to_path_buf());
        }
    }
    Err(Error::new(ErrorKind::NotFound, "No se encontró el archivo docker-compose.yml"))
}
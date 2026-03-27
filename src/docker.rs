use bollard::Docker;
use bollard::errors::Error as DockerError;
use bollard::query_parameters::ListImagesOptionsBuilder;
use walkdir::WalkDir;
use std::collections::btree_map::Entry;
use std::path::{self, PathBuf};
use std::path::Path;
use std::io::{Error, ErrorKind, empty};
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

pub fn file_is_here(file_path: &str, target: &str) -> bool {
    let path = Path::new(file_path);
    path.join(target).exists()
}

pub fn find_ochestor_folder(file_path: &str) -> std::io::Result<PathBuf> {

    let target = ".git";
    let mut contador = 0;
    let mut current_dir = PathBuf::from(file_path);

    loop {
        if file_is_here(current_dir.to_str().unwrap(), target) {
            return Ok(current_dir);
        }

        contador += 1;

        // 3. Si después de 5 carpetas no se encuentra el .git, el orquestador es inaccesible.
        if contador >= 5{
            break;
        }


        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => break,
        }
    }

    Err(Error::new(ErrorKind::NotFound, "No se encontró la carpeta raiz"))
}

pub async fn find_container_orchestrator (file_path: &String) -> std::io::Result<(PathBuf)>{

    // Pasos para encontrar el orquestador 
    let target = "docker-compose.yml";

    // 1. Comprobar en la dirección de orígen.
    if file_is_here(file_path, target) {
        return Ok(PathBuf::from(file_path))
    }


    // 2. Si no se encuentra, realizar Upward Discovery hasta encontrar el .git.

    let init_folder = find_ochestor_folder(file_path)?;


    // 4. Si encuentra el .git comprobar en la dirección actual de búsqueda.

    if file_is_here(init_folder.to_str().unwrap(), target) {
        return Ok(PathBuf::from(init_folder))
    }

    
    // 5. Si no encuentra el orquestador, realizar búsqueda mediante un método recursivo.

    for entry in WalkDir::new(init_folder.to_str().unwrap())
    .into_iter()
    .filter_map(|e| e.ok()) {

        if entry.file_name() == target {
            return Ok(entry.path().to_path_buf());
        }
    }

    // 6. Si no encuentra el orquestador, el orquestador es inaccesible.
    Err(Error::new(ErrorKind::NotFound, "No fue posible encontrar el orquestador"))
}
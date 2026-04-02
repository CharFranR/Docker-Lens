use bollard::Docker;
use bollard::errors::Error as DockerError;
use bollard::query_parameters::ListImagesOptionsBuilder;
use pyo3::sync::with_critical_section;
use walkdir::WalkDir;
use std::collections::btree_map::{Entry, Range};
use std::path::{self, PathBuf};
use std::path::Path;
use std::io::{Error, ErrorKind, empty};
use std::fs;
use std::slice::ArrayWindows;
use serde::{Deserialize, ser};
use std::collections::HashMap;
use bollard::query_parameters::ListContainersOptions;
use bollard::query_parameters::ListContainersOptionsBuilder;

// Public lists for heuristics
pub const NAME_LIST: &[&str] = &[
    "db", "database", "postgres", "postgresql", "pg", "pgsql", "postgres_db",
    "postgres-database", "postgres_service", "postgres-server", "postgresdb", "db-server",
    "db_postgres", "database_postgres", "main_db", "primary_db", "data", "data_db",
    "storage_db", "storage", "dbs", "database_service", "sql_db", "sql_database",
    "pg-master", "pg-replica", "database-main", "db-main", "primary-database",
] as &[&str];

pub const IMAGE_LIST: &[&str] = &[
    "postgres", "postgis", "postgresql", "timescaledb", "timescale", "bitnami/postgresql",
] as &[&str];

#[derive(Debug)]
pub struct DB_Data {
    pub port: String,
    pub POSTGRES_USER: String,
    pub POSTGRES_PASSWORD: String,
    pub POSTGRES_DB: String
}


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

pub async fn find_container_orchestrator (file_path: &String) -> std::io::Result<PathBuf>{

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


// Serializer docker

fn serializer_docker(docker_compose_text: String) -> Result<DockerCompose, Error> {
    let compose_data: DockerCompose = serde_yaml::from_str(&docker_compose_text)
    .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Error en el YAML: {}", e)))?;
    
    Ok(compose_data)
}


// New Feature: No es viable solo busca el servicio db, hay que implementar una cierta
// heuristica para encontrar el servicio de db independientemente del nombre


pub fn find_db_service (folder_path: &PathBuf) -> std::io::Result<DB_Data> {

    let orchestrator_path = folder_path.join("docker-compose.yml");
    let docker_compose_text = fs::read_to_string(orchestrator_path)?;

    let docker_data = serializer_docker(docker_compose_text)?;

    // Crear diccionario de servicios y puntos
    let mut servicios: HashMap<String, i32> = HashMap::new();


    for (name, service) in &docker_data.services {
        servicios.insert(name.clone(),0);
    }
    


    for (name, score) in servicios.iter_mut() {

        // 2.2 Puntos por nombre de servicio 
        if NAME_LIST.iter().any(|alias| name.eq_ignore_ascii_case(alias)) {
            *score += 10;
        }

        // Consultamos la definición del servicio en el compose
        if let Some(svc) = docker_data.services.get(name) {

            // 2.3 Puntos por (no tener) depends
            if svc.depends_on.is_none() {
                *score += 20;
            }

            // 2.4 Puntos por la image
            if let Some(img) = &svc.image {
                let img_norm = img.to_ascii_lowercase();
                if IMAGE_LIST.iter().any(|w| img_norm.contains(w)) {
                    *score += 30;
                } else if img_norm.contains("postgres") || img_norm.contains("postgis") {
                    *score += 10;
                }
            }

            // 2.5 Punts por puerto
            if let Some(ports_vec) = &svc.ports {
                for p in ports_vec {
                    let p_clean = p.split('/').next().unwrap_or(p);
                    let parts: Vec<&str> = p_clean.split(':').collect();
                    if parts.len() == 2 && parts[1] == "5432" {
                        *score += 25;
                        break;
                    } else if parts.len() == 3 && parts[2] == "5432" {
                        *score += 25;
                        break;
                    }
                }
            }

            // 2.6 Environment evidence (POSTGRES_*)
            if let Some(env_value) = &svc.environment {
                if let Some(map) = env_value.as_mapping() {
                    if map.get(&serde_yaml::Value::String("POSTGRES_DB".into())).is_some() {
                        *score += 20;
                    }
                    if map.get(&serde_yaml::Value::String("POSTGRES_USER".into())).is_some() {
                        *score += 15;
                    }
                    if map.get(&serde_yaml::Value::String("POSTGRES_PASSWORD".into())).is_some() {
                        *score += 15;
                    }
                } else if let Some(seq) = env_value.as_sequence() {
                    for item in seq {
                        if let Some(s) = item.as_str() {
                            if s.starts_with("POSTGRES_DB=") {
                                *score += 20;
                            } else if s.starts_with("POSTGRES_USER=") {
                                *score += 15;
                            } else if s.starts_with("POSTGRES_PASSWORD=") {
                                *score += 15;
                            }
                        }
                    }
                }
            }
        }
    }



    // Identificar servicio con más puntos
    let mut max_score = 0;
    let mut service_winner: Option<String> = None;

    for (name, score) in servicios.iter() {
        if *score > max_score {
            max_score = *score;
            service_winner = Some(name.clone());
        }
    }

    // Aislar el servicio ganador


    let service = match service_winner {
        Some(winner) => docker_data.services.get(&winner).unwrap(),
        None => return Err(Error::new(ErrorKind::NotFound, "No se encontró un servicio de base de datos")),
    };

    // Extaer datos del servicio ganador


    let mut port_winner = String::from("5432");
    let mut postgress_user_winner = String::from("postress");
    let mut postgress_password_winner = String::from("postgress");
    let mut postgress_db_winner = String::from("appdb");



    if let Some(port_temp) = &service.ports {
        port_winner = String::from(port_temp[0].split(':').last().unwrap_or("5432"));
    }
    
    if let Some(env_temp) = &service.environment {

        if let Some (map) = env_temp.as_mapping() {


        if let Some(postgress_user) = map.get(&serde_yaml::Value::String("POSTGRES_USER".into())) {
            if let Some(db_str) = postgress_user.as_str() {
                postgress_user_winner = String::from(db_str);
            }
        }

        if let Some(postgress_password) = map.get(&serde_yaml::Value::String("POSTGRES_PASSWORD".into())) {
            if let Some(db_str) = postgress_password.as_str() {
                postgress_password_winner = String::from(db_str);
            }
        }


        if let Some(postgres_db) = map.get(&serde_yaml::Value::String("POSTGRES_DB".into())) {
            if let Some(db_str) = postgres_db.as_str() {
                postgress_db_winner = String::from(db_str);
            }
        }
             
        }
    }


    let credential = DB_Data{
        port: String::from(port_winner),
        POSTGRES_USER: String::from(postgress_user_winner),
        POSTGRES_PASSWORD: String::from(postgress_password_winner),
        POSTGRES_DB: String::from(postgress_db_winner)
    };
    

   Ok(credential)

}

// 1. Listar contenedores y filtrar por nombre
pub async fn find_container_by_name(docker: &Docker, name: &str) -> Result<Container, DockerError> {
    let containers = docker.list_containers(Some(ListContainersOptions {
        all: true,
        ..Default::default()
    })).await?;
    
    // Filtrar por nombre que coincida con el container_name del servicio
}
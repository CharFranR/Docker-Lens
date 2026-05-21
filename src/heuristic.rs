use std::fs;                         
use std::io::{Error, ErrorKind};     
use std::path::PathBuf; 
use std::collections::HashMap;

use crate::compose::serializer_docker;
use crate::types::DbData;

// La puta calidad y que fokin codigo espagueti

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


pub fn find_db_service (folder_path: &PathBuf) -> std::io::Result<DbData> {

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
                    if map.get(&serde_yaml::Value::String("postgres_db".into())).is_some() {
                        *score += 20;
                    }
                    if map.get(&serde_yaml::Value::String("postgres_user".into())).is_some() {
                        *score += 15;
                    }
                    if map.get(&serde_yaml::Value::String("postgres_password".into())).is_some() {
                        *score += 15;
                    }
                } else if let Some(seq) = env_value.as_sequence() {
                    for item in seq {
                        if let Some(s) = item.as_str() {
                            if s.starts_with("postgres_db=") {
                                *score += 20;
                            } else if s.starts_with("postgres_user=") {
                                *score += 15;
                            } else if s.starts_with("postgres_password=") {
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
    let mut postgres_user_winner = String::from("postgres");
    let mut postgres_password_winner = String::from("postgres");
    let mut postgres_db_winner = String::from("appdb");



    if let Some(port_temp) = &service.ports {
        port_winner = String::from(port_temp[0].split(':').last().unwrap_or("5432"));
    }
    
    if let Some(env_temp) = &service.environment {

        if let Some (map) = env_temp.as_mapping() {


        if let Some(postgress_user) = map.get(&serde_yaml::Value::String("postgres_user".into())) {
            if let Some(db_str) = postgress_user.as_str() {
                postgres_user_winner = String::from(db_str);
            }
        }

        if let Some(postgress_password) = map.get(&serde_yaml::Value::String("postgres_password".into())) {
            if let Some(db_str) = postgress_password.as_str() {
                postgres_password_winner = String::from(db_str);
            }
        }

        if let Some(postgres_db) = map.get(&serde_yaml::Value::String("postgres_db".into())) {
            if let Some(db_str) = postgres_db.as_str() {
                postgres_db_winner = String::from(db_str);
            }
        }
    }
}


    let credential = DbData{
        port: String::from(port_winner),
        postgres_user: String::from(postgres_user_winner),
        postgres_password: String::from(postgres_password_winner),
        postgres_db: String::from(postgres_db_winner)
    };
    

   Ok(credential)

}
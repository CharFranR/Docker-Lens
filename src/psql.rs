use std::{io::{Error, ErrorKind}, process::Command};


use rusqlite::types::Value;

use crate::types::DbData;
use crate::types::TablaInfo;
use crate::types::ColumnaInfo;


pub fn get_db_container_ip(service_winner: &str) -> Option<String> {
    let output = match Command::new("docker")
        .args(["ps", "-a", "--format", "{{.ID}}|{{.Names}}"])
        .output()
    {
        Ok(o) => o,
        Err(_) => {
            eprintln!("Error: Docker is not installed or not in PATH.");
            return None;
        }
    };
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    for line in output_str.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            let container_id = parts[0];
            let container_name = parts[1];
            let name_clean = container_name.trim_start_matches('/');
            
            if name_clean.contains(service_winner) || container_name.contains(service_winner) {
                let inspect_output = match Command::new("docker")
                    .args(["inspect", container_id, "--format", "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}"])
                    .output()
                {
                    Ok(o) => o,
                    Err(_) => {
                        eprintln!("Error: Could not inspect container '{}'.", container_name);
                        return None;
                    }
                };
                
                let ip = String::from_utf8_lossy(&inspect_output.stdout).trim().to_string();
                return Some(ip);
            }
        }
    }
    
    None
}


pub fn list_tables(credentials: &DbData) -> std::io::Result<String>{

    let port = String::from("-p") + &credentials.port;
    let user = String::from("-U") + &credentials.postgres_user;
    let password =  &credentials.postgres_password;
    let db = String::from("-d") + &credentials.postgres_db;

    let output = Command::new("psql")
        .args([
            "-hlocalhost",
            port.as_str(),
            user.as_str(),
            db.as_str(),
            "-c", "\\dt",
            "-w"  // No pedir password
        ])
        .env("PGPASSWORD", password.as_str())
        .output()?;

    let all_tables = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(all_tables)
}

pub fn make_query(credentials: &DbData, query:&str) -> std::io::Result<String>{

    let port = String::from("-p") + &credentials.port;
    let user = String::from("-U") + &credentials.postgres_user;
    let password =  &credentials.postgres_password;
    let db = String::from("-d") + &credentials.postgres_db;

    let response = Command::new("psql")
        .args([
            "-hlocalhost",
            port.as_str(),
            user.as_str(),
            db.as_str(),
            "-c", query,
            "-w"  // No pedir password
        ])
        .env("PGPASSWORD", password.as_str())
        .output()?;

    if !response.status.success() {
        let stderr = String::from_utf8_lossy(&response.stderr);
        return Err(Error::new(ErrorKind::Other, stderr.to_string()));
    }

    let output = String::from_utf8_lossy(&response.stdout).to_string();

    Ok(output)
}

pub fn export_csv(credentials: &DbData, table: &str, file_path: &str) -> std::io::Result<()> {

    let file = std::fs::File::create(file_path)?;
    let port = format!("-p{}", credentials.port);
    let user = format!("-U{}", credentials.postgres_user);
    let db = format!("-d{}", credentials.postgres_db);

    let sql = format!("COPY (SELECT * FROM {table}) TO STDOUT WITH CSV HEADER");
    let mut child = Command::new("psql")
        .args([
            "-hlocalhost",
            &port,
            &user,
            &db,
            "-c",
            &sql,
            "-w",
        ])
        .env("PGPASSWORD", &credentials.postgres_password)
        .stdout(file)
        .spawn()?;

    let status = child.wait()?;

    if !status.success() {
        return Err(Error::new(ErrorKind::Other, "Error al exportar CSV"));
    }

    Ok(())
}


fn get_db_struct (credentials: &DbData) -> std::io::Result<Vec<String>> {
    
    // Listamos todas las tablas

    let tables_query = "SELECT tablename FROM pg_tables WHERE schemaname = 'public';";
    let raw_tables = make_query(credentials, tables_query)?;


    // Iteramos para obtener la estructura de toda la DB
    let mut db_struct: Vec<String> = Vec::new();


    for i in raw_tables.lines() {

        let line = i.trim();
        if line.is_empty() || line.starts_with("tablename") || line.starts_with('-') || line.starts_with('(') {
            continue;
        }

        let query = format!("SELECT column_name, data_type, is_nullable, column_default FROM information_schema.columns WHERE table_name = '{}' ORDER BY ordinal_position;", i.trim().to_string());
        
        match make_query(credentials, query.as_str()) {
            Ok(value) => {db_struct.push(value)} ,

            Err(e) => return  Err(e),
        };
    }

    Ok(db_struct)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DbData;
    #[test]
    fn test_get_db_struct_real() {
        let creds = DbData {
            port: "5432".into(),
            postgres_user: "postgres".into(),
            postgres_password: "postgres".into(),
            postgres_db: "appdb".into(),
        };
        let result = get_db_struct(&creds).unwrap();

        println!("{:#?}", result);

        assert!(!result.is_empty());
    }
}
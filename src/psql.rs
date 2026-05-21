use std::process::Command;


use crate::types::DbData;


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
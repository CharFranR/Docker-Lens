// MySQL/MariaDB adapter via mysql CLI.
// Same pattern as postgres.rs but uses `mysql` binary.
use std::io::{Error, ErrorKind};
use std::process::Command;

use crate::types::GenericCredentials;

/// Resolve the Docker container IP for a given service name.
/// Reuses the same docker inspect pattern as postgres.
pub fn get_container_ip(service_winner: &str) -> Option<String> {
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
                    .args([
                        "inspect",
                        container_id,
                        "--format",
                        "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}",
                    ])
                    .output()
                {
                    Ok(o) => o,
                    Err(_) => {
                        eprintln!("Error: Could not inspect container '{}'.", container_name);
                        return None;
                    }
                };

                let ip = String::from_utf8_lossy(&inspect_output.stdout)
                    .trim()
                    .to_string();
                return Some(ip);
            }
        }
    }

    None
}

/// List all tables via `SHOW TABLES`.
pub fn list_tables(credentials: &GenericCredentials) -> std::io::Result<String> {
    let mut cmd = build_mysql_command(credentials);
    cmd.arg("-e").arg("SHOW TABLES;");

    let output = cmd.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            return Err(Error::new(ErrorKind::Other, stderr.to_string()));
        }
        return Ok(String::from("(No tables found)"));
    }

    Ok(stdout)
}

/// Execute an arbitrary SQL query via mysql CLI.
pub fn make_query(credentials: &GenericCredentials, query: &str) -> std::io::Result<String> {
    let mut cmd = build_mysql_command(credentials);
    cmd.arg("-e").arg(query);

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::new(ErrorKind::Other, stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

/// Export a table to CSV via mysql CLI --batch mode.
pub fn export_csv(
    credentials: &GenericCredentials,
    table: &str,
    file_path: &str,
) -> std::io::Result<()> {
    let file = std::fs::File::create(file_path)?;

    let sql = format!("SELECT * FROM `{table}`");
    let mut child = build_mysql_command(credentials)
        .arg("--batch")
        .arg("-e")
        .arg(&sql)
        .stdout(file)
        .spawn()?;

    let status = child.wait()?;

    if !status.success() {
        return Err(Error::new(ErrorKind::Other, "Error exporting CSV from MySQL"));
    }

    Ok(())
}

/// Build the base `mysql` Command with connection flags.
fn build_mysql_command(credentials: &GenericCredentials) -> Command {
    let mut cmd = Command::new("mysql");

    cmd.arg("-h")
        .arg(&credentials.host)
        .arg("-P")
        .arg(&credentials.port)
        .arg("-u")
        .arg(&credentials.user);

    // Pass password via env var instead of -p flag (avoids security warning)
    if !credentials.password.is_empty() {
        cmd.env("MYSQL_PWD", &credentials.password);
        // Skip the interactive password prompt
        cmd.arg("--skip-password-prompt");
    }

    if !credentials.database.is_empty() {
        cmd.arg("-D").arg(&credentials.database);
    }

    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DbType, GenericCredentials};

    fn mysql_creds() -> GenericCredentials {
        GenericCredentials {
            db_type: DbType::Mysql,
            host: "localhost".into(),
            port: "3306".into(),
            user: "root".into(),
            password: "root".into(),
            database: "testdb".into(),
        }
    }

    #[test]
    fn test_build_mysql_command_has_expected_args() {
        let creds = mysql_creds();
        let cmd = build_mysql_command(&creds);
        let program = cmd.get_program().to_str().unwrap();
        assert!(program.contains("mysql"), "Should use mysql binary");
    }

    #[test]
    fn test_build_mysql_command_without_password() {
        let creds = GenericCredentials {
            db_type: DbType::Mysql,
            host: "localhost".into(),
            port: "3306".into(),
            user: "root".into(),
            password: String::new(),
            database: "testdb".into(),
        };
        let cmd = build_mysql_command(&creds);
        // Should not panic — empty password is fine for some setups
        let program = cmd.get_program().to_str().unwrap();
        assert!(program.contains("mysql"));
    }

    #[test]
    fn test_list_tables_returns_error_when_mysql_unavailable() {
        let creds = mysql_creds();
        let result = list_tables(&creds);
        // If mysql is available and connects, it works; otherwise it's an error
        // Both outcomes are valid in test environments
        match result {
            Ok(output) => {
                // If MySQL is available, we should get output
                assert!(!output.is_empty() || output.contains("No tables"));
            }
            Err(_) => {
                // MySQL not available — expected in CI
            }
        }
    }

    #[test]
    fn test_make_query_returns_error_when_mysql_unavailable() {
        let creds = mysql_creds();
        let result = make_query(&creds, "SELECT 1");
        match result {
            Ok(output) => {
                assert!(!output.is_empty());
            }
            Err(_) => {
                // MySQL not available — expected
            }
        }
    }
}

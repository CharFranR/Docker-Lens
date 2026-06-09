use std::io::{Error, ErrorKind};

use bollard::Docker;
use bollard::query_parameters::ListContainersOptionsBuilder;

/// Connect to the local Docker daemon.
fn connect() -> Result<Docker, Error> {
    Docker::connect_with_local_defaults()
        .map_err(|e| Error::new(ErrorKind::Other, format!("Docker connect: {e}")))
}

/// Resolve the Docker container IP for a given service name.
///
/// Lists all containers, finds the one whose name contains `service_name`,
/// and returns its IP address from the first network.
pub fn get_container_ip(service_name: &str) -> Option<String> {
    let rt = tokio::runtime::Runtime::new().ok()?;
    let sn = service_name.to_string();

    rt.block_on(async {
        let docker = connect().ok()?;

        let containers = docker
            .list_containers(Some(
                ListContainersOptionsBuilder::default()
                    .all(true)
                    .build(),
            ))
            .await
            .ok()?;

        // Find the container whose name matches the service
        let container = containers.iter().find(|c| {
            c.names.as_ref().map_or(false, |names| {
                names.iter().any(|name| {
                    let clean = name.trim_start_matches('/');
                    clean.contains(&sn)
                })
            })
        })?;

        let id = container.id.as_deref()?;

        // Inspect the container to get network settings
        let inspect = docker.inspect_container(id, None).await.ok()?;

        // Extract IP from the first network
        let networks = inspect.network_settings?.networks?;
        let (_name, settings) = networks.into_iter().next()?;

        settings.ip_address
    })
}

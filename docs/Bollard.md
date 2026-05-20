# Aprende Bollard — Guía Práctica para Docker-Lens

> Bollard es el cliente **asíncrono de Rust para la API de Docker**. Si querés listar contenedores, inspeccionarlos, ejecutar comandos dentro de ellos, o gestionar imágenes y redes desde Rust, Bollard es la herramienta. Esta guía te lleva desde "conectar al daemon" hasta poder obtener la IP de un contenedor y ejecutar queries PostgreSQL dentro de él.

---

## Índice

1. **¿Qué es Bollard y por qué existe?**
2. **Conceptos Fundamentales**
   - 2.1 Dependencias y setup
   - 2.2 Conexión al daemon de Docker
   - 2.3 Async/Await — todo es asíncrono
3. **Operaciones Básicas**
   - 3.1 Verificar conexión (docker version)
   - 3.2 Listar imágenes
   - 3.3 Listar contenedores
4. **Inspeccionar Contenedores**
   - 4.1 `inspect_container` — la función clave
   - 4.2 Estructura de `ContainerInspectResponse`
   - 4.3 Obtener la IP de un contenedor
   - 4.4 Obtener puertos mapeados
5. **Ejecutar Comandos dentro de Contenedores**
   - 5.1 Crear una exec instance
   - 5.2 Ejecutar y capturar output
   - 5.3 Caso práctico: ejecutar `psql` dentro del contenedor
6. **Caso Real: Docker-Lens**
   - 6.1 Estrategia de conexión a PostgreSQL
   - 6.2 Opción A: Conexión directa con tokio-postgres
   - 6.3 Flujo completo: del compose a la query
   - 6.4 Código integrado
7. **Tips y Gotchas**
8. **Resumen Rápido**

---

# 1. ¿Qué es Bollard y por qué existe?

## El problema que resuelve

Docker tiene una API REST que expone todo: contenedores, imágenes, redes, volúmenes. Podés llamarla con `curl`, pero en Rust querés algo tipado, asíncrono, y que maneje errores correctamente.

Sin Bollard, tendrías que hacer esto:

```rust
// ❌ Sin Bollard: HTTP manual, parseo manual, sin tipos
let response = reqwest::get("http://localhost/v1.41/containers/json?all=true").await?;
let body: serde_json::Value = response.json().await?;
// y ahora navegar un JSON genérico a mano...
```

Con Bollard:

```rust
// ✅ Con Bollard: tipado, async, errores manejados
let containers = docker.list_containers(Some(options)).await?;
for c in &containers {
    println!("{:?}", c.names);  // c.names es Option<Vec<String>>
}
```

## El nombre

"Bollard" = bolardo, esos postes de acero que se usan para amarrear barcos en el puerto. Un guiño a Docker (los contenedores vienen del transporte marítimo).

## Analogía

Bollard es como el SDK oficial de Docker pero en Rust. Si conocés `docker-py` (Python) o `dockerode` (Node.js), Bollard es lo mismo pero con las ventajas de Rust: tipos, async nativo con tokio, y zero-cost abstractions.

---

# 2. Conceptos Fundamentales

## 2.1 Dependencias y setup

```toml
# Cargo.toml
[dependencies]
bollard = "*"
tokio = { version = "1", features = ["full"] }
futures-util = "0.3"  # necesario para streams (exec output)
```

**`futures-util`**: se necesita para procesar el output de `exec` (es un stream, no un valor fijo).

## 2.2 Conexión al daemon de Docker

Bollard soporta tres formas de conexión:

```rust
use bollard::Docker;

// Opción 1: Unix socket (Linux/macOS) — LA MÁS COMÚN
let docker = Docker::connect_with_socket_defaults()?;

// Opción 2: Unix socket con path custom
let docker = Docker::connect_with_socket(
    "/var/run/docker.sock",
    120,  // timeout en segundos
    bollard::API_DEFAULT_VERSION,
)?;

// Opción 3: TCP (Docker Desktop en Windows, o Docker remoto)
let docker = Docker::connect_with_http(
    "localhost",
    2375,
    120,
    &bollard::API_DEFAULT_VERSION,
)?;
```

**En Docker-Lens ya usás `connect_with_local_defaults()`** — eso es un wrapper que intenta Unix socket y TCP automáticamente.

### ¿Cómo sabe Bollard dónde está Docker?

En Linux, Docker expone su API en `/var/run/docker.sock`. Bollard busca ese archivo automáticamente con `socket_defaults`. Si tu usuario no tiene permisos, necesitás agregarlo al grupo `docker`:

```bash
sudo usermod -aG docker $USER
# y cerrá/abrí sesión
```

## 2.3 Async/Await — todo es asíncrono

Cada llamada a Bollard es `async`. Necesitás un runtime tokio:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Docker::connect_with_socket_defaults()?;
    // ... todo es await acá
    Ok(())
}
```

**Regla**: si una función llama a Bollard, tiene que ser `async`.

---

# 3. Operaciones Básicas

## 3.1 Verificar conexión (docker version)

La forma más simple de saber si Docker está corriendo:

```rust
use bollard::Docker;

async fn check_docker() -> Result<(), Box<dyn std::error::Error>> {
    let docker = Docker::connect_with_socket_defaults()?;
    
    match docker.version().await {
        Ok(version) => {
            println!("Docker version: {:?}", version.version);
            println!("API version: {:?}", version.api_version);
            println!("OS: {:?}", version.os);
        }
        Err(e) => {
            eprintln!("Error conectando a Docker: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}
```

**En Docker-Lens ya tenés esto** en `docker_version()`.

## 3.2 Listar imágenes

```rust
use bollard::query_parameters::ListImagesOptionsBuilder;

async fn list_images(docker: &Docker) -> Result<(), Box<dyn std::error::Error>> {
    let options = ListImagesOptionsBuilder::default()
        .all(true)
        .build();

    let images = docker.list_images(Some(options)).await?;

    for image in &images {
        println!("Image: {:?}", image.repo_tags);
        println!("  ID: {}", image.id);
        println!("  Size: {} bytes", image.size.unwrap_or(0));
    }

    Ok(())
}
```

**En Docker-Lens ya tenés esto** en `docker_images()`.

## 3.3 Listar contenedores

```rust
use bollard::query_parameters::ListContainersOptionsBuilder;
use std::collections::HashMap;

async fn list_containers(docker: &Docker) -> Result<(), Box<dyn std::error::Error>> {
    // Listar TODOS los contenedores (incluyendo detenidos)
    let options = ListContainersOptionsBuilder::default()
        .all(true)
        .build();

    let containers = docker.list_containers(Some(options)).await?;

    for container in &containers {
        println!("ID: {}", container.id.as_ref().unwrap_or(&"N/A".into()));
        println!("Names: {:?}", container.names);
        println!("Image: {:?}", container.image);
        println!("State: {:?}", container.state);
        println!("Status: {:?}", container.status);
        println!("Ports: {:?}", container.ports);
        println!();
    }

    // Filtrar SOLO los que están corriendo
    let mut filters = HashMap::new();
    filters.insert("status".to_string(), vec!["running".to_string()]);

    let running_options = ListContainersOptionsBuilder::default()
        .all(false)
        .filters(&filters)
        .build();

    let running = docker.list_containers(Some(running_options)).await?;
    println!("Contenedores corriendo: {}", running.len());

    Ok(())
}
```

### Campos importantes de cada contenedor

| Campo | Tipo | Qué es |
|-------|------|--------|
| `id` | `Option<String>` | ID completo del contenedor (64 chars) |
| `names` | `Option<Vec<String>>` | Nombres del contenedor (`["/postgres_db"]`) |
| `image` | `Option<String>` | Imagen usada (`"postgres:16"`) |
| `state` | `Option<String>` | Estado (`"running"`, `"exited"`, etc.) |
| `status` | `Option<String>` | Status descriptivo (`"Up 2 hours"`) |
| `ports` | `Option<Vec<Port>>` | Puertos mapeados |

---

# 4. Inspeccionar Contenedores

## 4.1 `inspect_container` — la función clave

Esta es la función más importante para Docker-Lens. Te da TODA la información de un contenedor:

```rust
async fn inspect_container(
    docker: &Docker,
    container_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let info = docker.inspect_container(container_id, None).await?;

    println!("ID: {:?}", info.id);
    println!("Name: {:?}", info.name);
    println!("Image: {:?}", info.image);
    println!("State: {:?}", info.state);
    println!("Network: {:?}", info.network_settings);

    Ok(())
}
```

## 4.2 Estructura de `ContainerInspectResponse`

El objeto que devuelve `inspect_container` tiene muchos campos. Los más importantes para Docker-Lens:

```
ContainerInspectResponse
├── id: Option<String>
├── name: Option<String>
├── state: Option<ContainerState>
│   ├── running: Option<bool>
│   ├── status: Option<String>
│   └── ...
├── network_settings: Option<NetworkSettings>  ← ESTE ES EL QUE NECESITÁS
│   ├── ip_address: Option<String>
│   ├── gateway: Option<String>
│   ├── networks: Option<HashMap<String, EndpointSettings>>
│   │   └── [network_name]
│   │       └── ip_address: Option<String>  ← LA IP DEL CONTENEDOR
│   └── ports: Option<HashMap<String, Option<Vec<PortBinding>>>>
└── host_config: Option<HostConfig>
    └── port_bindings: Option<HashMap<String, Option<Vec<PortBinding>>>>
```

## 4.3 Obtener la IP de un contenedor

**Este es el paso clave para Docker-Lens.** Necesitás la IP para conectarte a PostgreSQL.

```rust
fn get_container_ip(info: &bollard::models::ContainerInspectResponse) -> Option<String> {
    let network_settings = info.network_settings.as_ref()?;
    
    // Opción 1: IP directa (a veces está vacía)
    if let Some(ip) = &network_settings.ip_address {
        if !ip.is_empty() {
            return Some(ip.clone());
        }
    }

    // Opción 2: IP desde las redes (más confiable)
    if let Some(networks) = &network_settings.networks {
        for (name, endpoint) in networks {
            if let Some(ip) = &endpoint.ip_address {
                if !ip.is_empty() {
                    println!("Red: {} → IP: {}", name, ip);
                    return Some(ip.clone());
                }
            }
        }
    }

    None
}
```

**¿Por qué hay dos formas?**

- `network_settings.ip_address` → IP "principal" del contenedor. A veces está vacía.
- `network_settings.networks[nombre].ip_address` → IP dentro de una red específica. Más confiable.

Un contenedor puede estar en múltiples redes (bridge, custom, etc.). Cada red le da una IP distinta.

### Ejemplo de output real

```
Red: bridge → IP: 172.17.0.2
Red: proyecto-asignatura-web_default → IP: 172.18.0.3
```

## 4.4 Obtener puertos mapeados

Para saber si un contenedor expone puertos al host:

```rust
fn get_mapped_ports(info: &bollard::models::ContainerInspectResponse) -> Vec<(String, String)> {
    let mut result = Vec::new();

    if let Some(port_bindings) = info
        .host_config
        .as_ref()
        .and_then(|hc| hc.port_bindings.as_ref())
    {
        for (container_port, bindings) in port_bindings {
            if let Some(bindings) = bindings {
                for binding in bindings {
                    let host_ip = binding.host_ip.as_deref().unwrap_or("0.0.0.0");
                    let host_port = binding.host_port.as_deref().unwrap_or("?");
                    result.push((
                        container_port.clone(),
                        format!("{}:{}", host_ip, host_port),
                    ));
                }
            }
        }
    }

    result
}

// Uso:
// [("5432/tcp", "0.0.0.0:5432")] → el puerto 5432 del contenedor está mapeado al host
```

---

# 5. Ejecutar Comandos dentro de Contenedores

## 5.1 Crear una exec instance

Docker te permite ejecutar comandos dentro de un contenedor corriendo. Bollard lo expone con `create_exec` + `start_exec`.

```rust
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use bollard_stubs::models::ExecConfig;
use futures_util::stream::StreamExt;

async fn run_in_container(
    docker: &Docker,
    container_id: &str,
    cmd: Vec<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    // 1. Crear la exec instance
    let exec_config = ExecConfig {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
        ..Default::default()
    };

    let exec = docker.create_exec(container_id, exec_config).await?;

    // 2. Ejecutar y capturar output
    let start_config = StartExecOptions {
        detach: false,
        ..Default::default()
    };

    let mut output = String::new();

    match docker.start_exec(&exec.id, Some(start_config)).await? {
        StartExecResults::Attached { mut output: stream, .. } => {
            while let Some(Ok(msg)) = stream.next().await {
                output.push_str(&msg.to_string());
            }
        }
        StartExecResults::Detached => {
            return Err("Exec se detachó inesperadamente".into());
        }
    }

    // 3. Verificar exit code
    let inspect = docker.inspect_exec(&exec.id).await?;
    if inspect.exit_code != Some(0) {
        return Err(format!("Comando falló con exit code: {:?}", inspect.exit_code).into());
    }

    Ok(output)
}
```

## 5.2 Ejecutar y capturar output

La función anterior devuelve un `String` con todo el output. Ejemplo de uso:

```rust
// Listar archivos en /
let output = run_in_container(&docker, "mi-contenedor", vec!["ls", "-la", "/"]).await?;
println!("{}", output);

// Ejecutar un comando en bash
let output = run_in_container(
    &docker,
    "mi-contenedor",
    vec!["sh", "-c", "echo hola && date"]
).await?;
println!("{}", output);
```

## 5.3 Caso práctico: ejecutar `psql` dentro del contenedor

Para Docker-Lens, la forma más fácil de ejecutar queries sin exponer puertos es ejecutar `psql` dentro del contenedor de PostgreSQL:

```rust
async fn run_psql_query(
    docker: &Docker,
    container_id: &str,
    db_user: &str,
    db_name: &str,
    query: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let cmd = vec![
        "psql",
        "-U", db_user,
        "-d", db_name,
        "-c", query,
    ];

    run_in_container(docker, container_id, cmd).await
}
```

**Esto es BRILLANTE para Docker-Lens** porque:

- No necesitás mapear puertos
- No necesitás obtener la IP del contenedor
- No necesitás `tokio-postgres` ni configurar TLS
- Docker ya tiene acceso al socket interno de PostgreSQL

---

# 6. Caso Real: Docker-Lens

## 6.1 Estrategia de conexión a PostgreSQL

Tenés dos caminos para conectarte a la DB:

```
┌─────────────────────────────────────────────────┐
│              Opción A: Puerto mapeado            │
│                                                   │
│  compose dice: ports: ["5432:5432"]              │
│  → Conectás a localhost:5432 con tokio-postgres  │
│  → Simple, directo                               │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│          Opción B: exec dentro del contenedor     │
│                                                   │
│  No importa si hay puerto mapeado o no           │
│  → Ejecutás psql directamente via Docker API     │
│  → No necesitás tokio-postgres                    │
│  → Más robusto, funciona siempre                 │
└─────────────────────────────────────────────────┘
```

**Mi consejo: Opción B** para el MVP general. Pero si tu compose tiene el puerto mapeado, **Opción A con `tokio-postgres` es más directa** — queries tipadas, sin parsear texto.

## 6.2 Opción A: Conexión directa con tokio-postgres

### Dependencia

```toml
# Cargo.toml
tokio-postgres = "0.7"
```

### ¿Qué es `tokio-postgres`?

Es el cliente **asíncrono de Rust para PostgreSQL**. Habla directamente con la DB sin pasar por Docker. Funciona con tokio (que ya usás).

### Conectar

```rust
use tokio_postgres::{NoTls, Client};

async fn connect_postgres(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    db_name: &str,
) -> Result<Client, Box<dyn std::error::Error>> {
    let connection_string = format!(
        "host={} port={} user={} password={} dbname={}",
        host, port, user, password, db_name
    );

    let (client, connection) = tokio_postgres::connect(&connection_string, NoTls).await?;

    // La conexión vive en un task separado
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Error de conexión PostgreSQL: {}", e);
        }
    });

    Ok(client)
}
```

**¿Qué es `NoTls`?** Le dice que no use SSL/TLS. Para conexiones locales de Docker está perfecto. Si necesitás TLS, cambiás por el feature `tls` de tokio-postgres.

**¿Por qué `tokio::spawn(connection)`?** La conexión es un futuro que maneja el protocolo internamente. Si no lo spawneás, la conexión se cierra inmediatamente.

### Ejecutar queries

```rust
// Query simple — obtener tablas
async fn get_tables(client: &Client) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let rows = client.query(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name",
        &[],
    ).await?;

    let tables: Vec<String> = rows.iter()
        .map(|row| row.get(0))
        .collect();

    Ok(tables)
}

// Query con parámetros (seguro contra SQL injection)
async fn get_rows(
    client: &Client,
    table: &str,
    limit: i64,
) -> Result<Vec<tokio_postgres::Row>, Box<dyn std::error::Error>> {
    let query = format!("SELECT * FROM {} LIMIT $1", table);
    // Nota: el nombre de tabla no se puede parametrizar con $1
    // Solo los valores se parametrizan
    
    let rows = client.query(&query, &[&limit]).await?;
    Ok(rows)
}
```

### Leer valores de un Row

```rust
// Los valores se leen por índice de columna o por nombre
for row in &rows {
    let id: i32 = row.get(0);           // por índice
    let name: &str = row.get("name");   // por nombre (si la query tiene nombre)
    let email: Option<String> = row.get(2);  // puede ser NULL
}
```

**Tipos comunes de lectura:**

| Tipo PostgreSQL | Tipo Rust |
|-----------------|-----------|
| `INTEGER` | `i32`, `i64` |
| `VARCHAR/TEXT` | `String`, `&str` |
| `BOOLEAN` | `bool` |
| `TIMESTAMP` | `SystemTime`, `NaiveDateTime` (con chrono) |
| `JSONB` | `serde_json::Value` (con feature json) |
| `NULL` | `Option<T>` |

### Ejemplo completo: listar tablas con registros

```rust
async fn dump_database_via_postgres(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    db_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Conectar
    let client = connect_postgres(host, port, user, password, db_name).await?;
    println!("Conectado a PostgreSQL");

    // 2. Obtener tablas
    let tables = get_tables(&client).await?;
    println!("Tablas encontradas: {}", tables.len());

    // 3. Para cada tabla, mostrar primeros 5 registros
    for table in &tables {
        println!("\n=== {} ===", table);

        let rows = client.query(
            &format!("SELECT * FROM {} LIMIT 5", table),
            &[],
        ).await?;

        if rows.is_empty() {
            println!("  (vacía)");
            continue;
        }

        // Imprimir nombres de columnas (si querés)
        let columns: Vec<String> = rows[0].columns()
            .iter()
            .map(|c| c.name().to_string())
            .collect();
        println!("  Columnas: {}", columns.join(" | "));

        // Imprimir filas
        for row in &rows {
            let values: Vec<String> = (0..row.len())
                .map(|i| {
                    // Intentar leer como String, si falla como debug
                    row.try_get::<_, Option<String>>(i)
                        .unwrap_or(None)
                        .unwrap_or_else(|| "NULL".to_string())
                })
                .collect();
            println!("  {}", values.join(" | "));
        }
    }

    Ok(())
}
```

### ¿Cómo sabés a qué host conectarte?

Acá es donde Bollard te ayuda. Tenés dos escenarios:

```
Escenario A: Puerto mapeado
  compose: ports: ["5432:5432"]
  → host = "localhost", port = 5432

Escenario B: Sin puerto mapeado (o mapeo raro)
  → Primero: docker.inspect_container() para obtener la IP interna
  → host = "172.17.0.2", port = 5432
```

```rust
// Resolver host y puerto
async fn resolve_connection(
    docker: &Docker,
    container_id: &str,
    container_port: &str,  // "5432"
) -> Result<(String, u16), Box<dyn std::error::Error>> {
    let info = docker.inspect_container(container_id, None).await?;

    // Ver si hay puerto mapeado al host
    if let Some(port_bindings) = info
        .host_config
        .as_ref()
        .and_then(|hc| hc.port_bindings.as_ref())
    {
        let key = format!("{}/tcp", container_port);
        if let Some(bindings) = port_bindings.get(&key) {
            if let Some(bindings) = bindings {
                if let Some(binding) = bindings.first() {
                    let host_port: u16 = binding.host_port
                        .as_ref()
                        .ok_or("Sin host_port")?
                        .parse()?;
                    return Ok(("localhost".to_string(), host_port));
                }
            }
        }
    }

    // Si no hay mapeo, usar IP interna
    let ip = get_container_ip(&info)
        .ok_or("No se pudo obtener IP del contenedor")?;
    let port: u16 = container_port.parse()?;

    Ok((ip, port))
}
```

### Ventajas de tokio-postgres vs exec

| | tokio-postgres | exec psql |
|---|---|---|
| **Output** | Tipado (`Row`, columnas) | Texto plano (hay que parsear) |
| **Velocidad** | Más rápido (protocolo nativo) | Más lento (spawn de proceso) |
| **Dependencias** | Necesita `tokio-postgres` | Solo `bollard` |
| **Requisitos** | Puerto accesible o IP | Contenedor corriendo |
| **SQL injection** | Parámetros seguros ($1, $2) | Formato directo |

**Para tu MVP con puerto mapeado: tokio-postgres es la mejor opción.**

---

## 6.3 Flujo completo: del compose a la query

```
1. find_db_service()
   └─ Encontrar el servicio de DB en docker-compose.yml
   └─ Extraer: container_name, postgres_user, postgres_db

2. find_container_by_name(docker, container_name)
   └─ Listar contenedores corriendo
   └─ Buscar el que coincida con container_name
   └─ Obtener su container_id

3. run_psql_query(docker, container_id, user, db, "SELECT ...")
   └─ Ejecutar psql dentro del contenedor
   └─ Capturar output

4. parsear_output_y_mostrar()
   └─ Procesar el texto de psql
   └─ Imprimir tablas y registros
```

## 6.4 Código integrado (esqueleto)

```rust
// === Paso 1: Encontrar el contenedor por nombre ===

async fn find_container_id(
    docker: &Docker,
    container_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    use bollard::query_parameters::ListContainersOptionsBuilder;
    use std::collections::HashMap;

    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec![container_name.to_string()]);

    let options = ListContainersOptionsBuilder::default()
        .all(true)
        .filters(&filters)
        .build();

    let containers = docker.list_containers(Some(options)).await?;

    match containers.first() {
        Some(c) => {
            let id = c.id.as_ref()
                .ok_or("Contenedor sin ID")?
                .clone();
            Ok(id)
        }
        None => Err(format!("No se encontró contenedor con nombre: {}", container_name).into()),
    }
}

// === Paso 2: Obtener todas las tablas ===

async fn get_tables(
    docker: &Docker,
    container_id: &str,
    db_user: &str,
    db_name: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let query = "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name;";
    
    let output = run_psql_query(docker, container_id, db_user, db_name, query).await?;
    
    // Parsear output de psql (es texto plano con formato de tabla)
    // Ejemplo de output:
    //  table_name
    // ------------
    //  users
    //  products
    //  orders
    // (3 rows)
    
    let tables: Vec<String> = output
        .lines()
        .skip(2)  // saltar header y separador
        .filter(|line| !line.trim().is_empty() && !line.contains("rows)"))
        .map(|line| line.trim().to_string())
        .collect();

    Ok(tables)
}

// === Paso 3: Obtener primeros N registros de una tabla ===

async fn get_rows(
    docker: &Docker,
    container_id: &str,
    db_user: &str,
    db_name: &str,
    table: &str,
    limit: i32,
) -> Result<String, Box<dyn std::error::Error>> {
    let query = format!("SELECT * FROM {} LIMIT {};", table, limit);
    run_psql_query(docker, container_id, db_user, db_name, &query).await
}

// === Flujo principal ===

async fn dump_database(
    docker: &Docker,
    db_data: &DBData,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Encontrar el contenedor
    let container_id = find_container_id(docker, &db_data.container_name).await?;
    println!("Contenedor encontrado: {}", container_id);

    // 2. Listar tablas
    let tables = get_tables(docker, &container_id, &db_data.postgres_user, &db_data.postgres_db).await?;
    println!("Tablas encontradas: {}", tables.len());

    // 3. Mostrar registros de cada tabla
    for table in &tables {
        println!("\n=== {} ===", table);
        let rows = get_rows(docker, &container_id, &db_data.postgres_user, &db_data.postgres_db, table, 5).await?;
        println!("{}", rows);
    }

    Ok(())
}
```

---

# 7. Tips y Gotchas

## Tip 1: Los IDs de contenedor pueden ser largos o cortos
Docker acepta tanto el ID completo (64 chars) como el prefijo (12 chars). Los nombres con `/` al inicio también funcionan: `"/postgres_db"`.

## Tip 2: `list_containers` no devuelve todos por default
Necesitás `.all(true)` para incluir los detenidos. Sin eso, solo ves los que están corriendo.

## Tip 3: Los filtros son case-sensitive
`filters.insert("name".to_string(), vec!["Postgres"])` NO va a encontrar `"postgres"`. Usá el nombre exacto del contenedor.

## Tip 4: `create_exec` con `sh -c` para comandos complejos
Si necesitás pipes, redirects, o múltiples comandos:
```rust
cmd: Some(vec!["sh".into(), "-c".into(), "psql -U user -d db -c 'SELECT 1'".into()])
```

## Tip 5: El output de `psql` tiene formato de tabla
La salida de psql viene con headers, separadores, y el conteo de filas:
```
 table_name 
------------
 users
(1 row)
```
Necesitás parsear este texto para extraer solo los datos.

## Tip 6: `psql -t` para output limpio
Usá la flag `-t` (tuples only) para quitar headers:
```rust
vec!["psql", "-U", user, "-d", db, "-t", "-c", query]
// Output: "users" (sin headers ni separadores)
```

## Tip 7: `psql -A` para output sin padding
Combinado con `-t`, te da datos crudos:
```rust
vec!["psql", "-U", user, "-d", db, "-t", "-A", "-c", query]
```

## Tip 8: Errores de exec no siempre son obvios
Si `psql` falla (DB no existe, usuario incorrecto), el error viene por stderr. Asegurate de capturar ambos: `attach_stdout: true` y `attach_stderr: true`.

## Tip 9: Timeout en exec
Si una query tarda mucho, `start_exec` puede colgar. Considerá usar `tokio::time::timeout`:
```rust
use tokio::time::{timeout, Duration};

let result = timeout(
    Duration::from_secs(10),
    run_psql_query(&docker, &id, user, db, query)
).await?;
```

---

# 8. Resumen Rápido

### Conceptos clave

| Concepto | Qué es |
|----------|--------|
| **`Docker::connect_with_socket_defaults()`** | Conectar al daemon local |
| **`docker.list_containers()`** | Listar contenedores (con filtros) |
| **`docker.inspect_container()`** | Obtener info detallada de un contenedor |
| **`docker.create_exec()` + `start_exec()`** | Ejecutar comandos dentro de un contenedor |
| **`ContainerInspectResponse.network_settings`** | IP del contenedor, redes, puertos |
| **`ExecConfig`** | Configuración del comando a ejecutar |
| **`StartExecResults::Attached`** | Output capturado del comando |

### Las dos formas de acceder a PostgreSQL

| Método | Ventaja | Desventaja |
|--------|---------|------------|
| **tokio-postgres + IP/puerto** | Conexión directa, queries tipadas | Necesitás puerto mapeado o IP interna |
| **exec psql via Docker API** | Siempre funciona, sin dependencias extra | Output es texto plano, hay que parsear |

### Patrón para Docker-Lens

```
docker-compose.yml
  └─ find_db_service() → DB_Data { container_name, user, db }
       └─ find_container_id(docker, container_name)
            └─ docker.list_containers(filters={"name": container_name})
                 └─ container_id
                      └─ run_psql_query(docker, container_id, user, db, query)
                           └─ docker.create_exec() + start_exec()
                                └─ output como String → parsear
```

### Dependencias necesarias

```toml
[dependencies]
bollard = "*"
tokio = { version = "1", features = ["full"] }
futures-util = "0.3"
```

---

# Apéndice: Referencia de métodos de Bollard

### Conexión
```rust
Docker::connect_with_local_defaults()     // auto-detecta Unix/TCP
Docker::connect_with_socket_defaults()    // solo Unix socket
Docker::connect_with_http_defaults()      // solo TCP
```

### Info del sistema
```rust
docker.version().await?           // VersionResponse
docker.info().await?              // SystemInfo
docker.ping().await?              // String ("OK")
```

### Contenedores
```rust
docker.list_containers(options).await?    // Vec<ContainerSummary>
docker.inspect_container(id, None).await? // ContainerInspectResponse
docker.start_container(id, None).await?
docker.stop_container(id, None).await?
docker.restart_container(id, None).await?
docker.remove_container(id, None).await?
```

### Exec (ejecutar comandos)
```rust
docker.create_exec(id, config).await?         // CreateExecResults { id }
docker.start_exec(exec_id, options).await?    // StartExecResults
docker.inspect_exec(exec_id).await?           // ExecInspectResponse { exit_code }
```

### Imágenes
```rust
docker.list_images(options).await?            // Vec<ImageSummary>
docker.inspect_image(name).await?             // ImageInspect
docker.pull_image(options, credentials).await? // Stream de progreso
```

### Redes
```rust
docker.list_networks::<String>(None).await?   // Vec<Network>
docker.inspect_network(id, None).await?       // NetworkInspect
```

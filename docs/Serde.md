# Aprende Serde — Guía Práctica para Docker-Lens

> Serde es la librería de **serialización y deserialización** más importante del ecosistema Rust. Si leés archivos YAML, JSON, TOML, o comunicás datos entre servicios, Serde es inevitable. Esta guía te lleva desde "¿qué es?" hasta poder parsear docker-compose.yml como un profesional.

---

## Índice

1. **¿Qué es Serde y por qué existe?**
2. **Conceptos Fundamentales**
   - 2.1 Serialización vs Deserialización
   - 2.2 El trait `Serialize` y `Deserialize`
   - 2.3 `#[derive(Serialize, Deserialize)]` — la magia automática
3. **Serde en la Práctica: Docker-Lens**
   - 3.1 Parsear un docker-compose.yml completo
   - 3.2 Campos opcionales con `Option<T>`
   - 3.3 El tipo `serde_yaml::Value` — cuando no sabés qué viene
   - 3.4 Extraer datos de `Value` — as_mapping(), as_str(), as_sequence()
4. **Formatos Soportados**
   - 4.1 YAML con `serde_yaml`
   - 4.2 JSON con `serde_json`
   - 4.3 Otros: TOML, CSV, etc.
5. **Atributos de Serde**
   - 5.1 `#[serde(default)]`
   - 5.2 `#[serde(rename = "...")]`
   - 5.3 `#[serde(skip)]`
   - 5.4 `#[serde(flatten)]`
6. **Caso Real: Environment Variables del Compose**
   - 6.1 Formato Mapa vs Formato Lista
   - 6.2 Patrón: dos capas de desestructuración
   - 6.3 Código completo de extracción
7. **Tips y Gotchas**
8. **Resumen Rápido**

---

# 1. ¿Qué es Serde y por qué existe?

## El problema que resuelve

Tenés un archivo YAML, JSON, o cualquier formato de texto con datos estructurados. Necesitás convertir eso a **tipos de Rust** (structs, enums, HashMaps). Y viceversa.

Sin Serde, tendrías que hacer esto a mano:

```rust
// ❌ Sin Serde: parseo manual, propenso a errores
let nombre = yaml_lines[0].split(':').nth(1).unwrap().trim();
let puerto = yaml_lines[1].split(':').nth(1).unwrap().trim().parse::<u16>().unwrap();
// ... y así con cada campo. Pesadilla.
```

Con Serde:

```rust
// ✅ Con Serde: declarás tu tipo y listo
#[derive(Deserialize)]
struct Config {
    nombre: String,
    puerto: u16,
}

let config: Config = serde_yaml::from_str(&yaml_text).unwrap();
```

**Serde hace el trabajo sucio por vos.** Analiza el texto, mapea cada campo a tu struct, valida tipos, y si algo falla, te dice exactamente qué.

## El nombre

**Ser**ialization + **De**serialization = **Serde**.

## Analogía de construcción

Pensalo así:

- **Tu struct** es el plano de una casa (dormitorio, cocina, baño)
- **El YAML/JSON** es la descripción textual de una casa
- **Serde** es el albañil que lee la descripción y construye la casa según el plano

Si la descripción dice "cocina: grande" pero tu plano espera un número, Serde te avisa antes de construir cualquier cosa.

---

# 2. Conceptos Fundamentales

## 2.1 Serialización vs Deserialización

```
┌─────────────┐                    ┌─────────────┐
│   Rust      │   Serializar      │   Texto     │
│   Struct    │ ──────────────►   │   YAML/JSON │
│             │   (escribir)      │             │
└─────────────┘                    └─────────────┘

┌─────────────┐                    ┌─────────────┐
│   Rust      │   Deserializar    │   Texto     │
│   Struct    │ ◄──────────────   │   YAML/JSON │
│             │   (leer)          │             │
└─────────────┘                    └─────────────┘
```

| Dirección | Qué hace | Función típica |
|-----------|----------|---------------|
| Struct → Texto | Serializar | `serde_yaml::to_string()` / `serde_json::to_string()` |
| Texto → Struct | Deserializar | `serde_yaml::from_str()` / `serde_json::from_str()` |

## 2.2 Los traits `Serialize` y `Deserialize`

Serde define dos traits:

```rust
pub trait Serialize {
    // "cómo convertirme a texto"
}

pub trait Deserialize<'de> {
    // "cómo construirme desde texto"
}
```

La mayoría de las veces NO los implementás manualmente. Usás `#[derive(...)]` y Serde genera el código por vos.

## 2.3 `#[derive(Serialize, Deserialize)]` — la magia automática

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct Service {
    container_name: Option<String>,
    image: Option<String>,
    ports: Option<Vec<String>>,
}
```

Con ese `derive`, Serde genera automáticamente:

- Cómo leer un YAML/JSON y crear un `Service`
- Cómo tomar un `Service` y escribirlo como YAML/JSON

**Regla**: casi siempre derivás AMBOS (`Serialize` + `Deserialize`). Solo deserializar es raro, pero pasa cuando solo leés datos y nunca los escribís.

---

# 3. Serde en la Práctica: Docker-Lens

## 3.1 Parsear un docker-compose.yml completo

Este es el caso real de Docker-Lens. Tenés un `docker-compose.yml`:

```yaml
services:
  db:
    container_name: mi-postgres
    image: postgres:16
    ports:
      - "5432:5432"
    environment:
      POSTGRES_DB: mi_base
      POSTGRES_USER: admin
      POSTGRES_PASSWORD: secret123
  web:
    image: node:20
    ports:
      - "3000:3000"
    depends_on:
      - db
```

Y necesitás leerlo en Rust. Primero, modelás el YAML como structs:

```rust
use serde::Deserialize;
use std::collections::HashMap;

// El nivel raíz del compose
#[derive(Debug, Deserialize)]
pub struct DockerCompose {
    pub services: HashMap<String, Service>,
}

// Cada servicio (db, web, etc.)
#[derive(Debug, Deserialize)]
pub struct Service {
    pub container_name: Option<String>,
    pub image: Option<String>,
    pub ports: Option<Vec<String>>,
    pub volumes: Option<Vec<String>>,
    pub environment: Option<serde_yaml::Value>,
    pub depends_on: Option<serde_yaml::Value>,
}
```

Después, parseás:

```rust
fn parse_compose(yaml_text: &str) -> Result<DockerCompose, Box<dyn std::error::Error>> {
    let compose: DockerCompose = serde_yaml::from_str(yaml_text)?;
    Ok(compose)
}
```

Y listo. `compose.services` es un `HashMap<String, Service>` donde las claves son `"db"`, `"web"`, etc.

**Tip clave**: el nombre del campo en el struct debe coincidir con el nombre en el YAML. Si en YAML dice `container_name`, en Rust debe ser `container_name` (o usás `#[serde(rename)]`).

## 3.2 Campos opcionales con `Option<T>`

En un docker-compose, casi todo es opcional. Un servicio puede no tener `ports`, o `volumes`, o `environment`.

```rust
#[derive(Debug, Deserialize)]
pub struct Service {
    pub image: Option<String>,          // puede no tener image (si usa build)
    pub ports: Option<Vec<String>>,     // puede no exponer puertos
    pub volumes: Option<Vec<String>>,   // puede no tener volúmenes
    pub environment: Option<serde_yaml::Value>,  // puede no tener env vars
}
```

**¿Qué pasa si el YAML no tiene ese campo?**

- Si el campo es `Option<T>` → se pone `None` automáticamente ✅
- Si el campo es `T` (no Option) → **ERROR de deserialización** ❌

**Regla práctica**: en YAMLs externos (configs, compose files), hacé TODOS los campos `Option<T>`. No sabés qué va a tener el usuario.

### `#[serde(default)]` como alternativa

```rust
#[derive(Debug, Deserialize)]
pub struct Service {
    pub image: Option<String>,
    
    #[serde(default)]
    pub ports: Vec<String>,  // si no existe en YAML, queda Vec vacío en vez de error
}
```

`#[serde(default)]` le dice a Serde: "si este campo no existe en el YAML, usá el valor por defecto del tipo". Para `Vec<String>`, eso es un `Vec` vacío.

## 3.3 El tipo `serde_yaml::Value` — cuando no sabés qué viene

Este es el concepto clave que te trabó. Hay veces que un campo en el YAML puede tener **múltiples formatos**.

El caso clásico en docker-compose: `environment` y `depends_on`.

```yaml
# Formato A: Mapa (key: value)
environment:
  POSTGRES_DB: mi_base
  POSTGRES_USER: admin

# Formato B: Lista (strings "KEY=VALUE")
environment:
  - POSTGRES_DB=mi_base
  - POSTGRES_USER=admin
```

**Ambos son válidos** según la especificación de Docker Compose. No podés modelarlo con un tipo fijo. Ahí es donde entra `serde_yaml::Value`.

### ¿Qué es `serde_yaml::Value`?

Es un enum que puede representar CUALQUIER valor YAML:

```rust
// Internamente es algo así (simplificado):
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Sequence(Vec<Value>),     // una lista
    Mapping(BTreeMap<Value, Value>),  // un mapa clave-valor
}
```

Cuando le decís a Serde que un campo es `serde_yaml::Value`, le estás diciendo: **"no intentes convertir esto a un tipo específico, guardalo como está y yo lo proceso después"**.

```rust
#[derive(Debug, Deserialize)]
pub struct Service {
    // No decimos HashMap<String, String> ni Vec<String>
    // Decimos "Value" porque puede ser cualquiera de los dos formatos
    pub environment: Option<serde_yaml::Value>,
    pub depends_on: Option<serde_yaml::Value>,
}
```

### Analogía

Es como decirle a alguien: "agarrá este paquete, no lo abras todavía, guardalo tal cual viene". Después vos lo abrís y ves qué hay adentro.

## 3.4 Extraer datos de `Value` — as_mapping(), as_str(), as_sequence()

Una vez que tenés un `Value`, necesitás sacar los datos reales. Serde te da métodos para "adivinar" qué tipo es:

### Métodos principales

| Método | Devuelve | Cuándo funciona |
|--------|----------|-----------------|
| `.as_bool()` | `Option<bool>` | Si el Value es un boolean |
| `.as_i64()` | `Option<i64>` | Si el Value es un número entero |
| `.as_f64()` | `Option<f64>` | Si el Value es un número decimal |
| `.as_str()` | `Option<&str>` | Si el Value es un string |
| `.as_sequence()` | `Option<&Vec<Value>>` | Si el Value es una lista |
| `.as_mapping()` | `Option<&BTreeMap<Value, Value>>` | Si el Value es un mapa |

**Cómo funcionan**: si el Value coincide con el tipo, devuelve `Some(valor)`. Si no, `None`.

### Ejemplo visual

```rust
// Si environment es un MAPA:
// { POSTGRES_DB: "mi_base", POSTGRES_USER: "admin" }

let env_value: serde_yaml::Value = service.environment.unwrap();

env_value.as_mapping()  // → Some(BTreeMap { ... }) ✅
env_value.as_sequence() // → None ❌
env_value.as_str()      // → None ❌

// Si environment es una LISTA:
// ["POSTGRES_DB=mi_base", "POSTGRES_USER=admin"]

env_value.as_sequence() // → Some(Vec [ ... ]) ✅
env_value.as_mapping()  // → None ❌
```

### Acceder a valores dentro de un Mapping

```rust
if let Some(map) = env_value.as_mapping() {
    // Para buscar por clave, necesitás un Value como clave
    let clave = serde_yaml::Value::String("POSTGRES_DB".into());
    
    if let Some(valor) = map.get(&clave) {
        // valor es otro Value — hay que sacar el string
        if let Some(db_name) = valor.as_str() {
            println!("Base de datos: {}", db_name);
        }
    }
}
```

¿Ves las dos capas?

```
map.get("POSTGRES_DB")   → Option<&Value>   (primera caja)
valor.as_str()           → Option<&str>     (segunda caja)
```

Primero sacás el Value del mapa, después sacás el string del Value. Son dos `if let` anidados.

---

# 4. Formatos Soportados

Serde es **agnóstico de formato**. Vos derivás `Serialize`/`Deserialize` una vez, y funciona con cualquier formato.

## 4.1 YAML con `serde_yaml`

```toml
# Cargo.toml
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
```

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    name: String,
    port: u16,
}

fn main() {
    let yaml = r#"
name: docker-lens
port: 5432
"#;

    let config: Config = serde_yaml::from_str(yaml).unwrap();
    println!("{:?}", config);
}
```

**Docker-Lens usa esto** porque docker-compose.yml es YAML.

## 4.2 JSON con `serde_json`

```toml
# Cargo.toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    name: String,
    age: u32,
}

fn main() {
    // JSON → Struct
    let json = r#"{"name": "Franco", "age": 25}"#;
    let user: User = serde_json::from_str(json).unwrap();

    // Struct → JSON
    let output = serde_json::to_string_pretty(&user).unwrap();
    println!("{}", output);
}
```

## 4.3 Otros formatos

| Formato | Crate | Ejemplo de uso |
|---------|-------|----------------|
| TOML | `toml` | Archivos `Cargo.toml`, configs |
| CSV | `csv` | Exportar datos tabulares |
| MessagePack | `rmp-serde` | Serialización binaria eficiente |
| RON | `ron` | "Rusty Object Notation" — como JSON pero con sintaxis Rust |

La clave: **mismo struct, distinto crate de formato**. Tu modelo de datos no cambia.

---

# 5. Atributos de Serde

Los atributos (`#[serde(...)]`) te dan control fino sobre cómo Serde maneja cada campo.

## 5.1 `#[serde(default)]`

Si el campo no existe en el input, usa el valor por defecto del tipo en vez de dar error.

```rust
#[derive(Debug, Deserialize)]
struct Service {
    image: Option<String>,
    
    #[serde(default)]
    ports: Vec<String>,  // si no existe → Vec vacío
}
```

Valores por defecto comunes:

| Tipo | Default |
|------|---------|
| `Option<T>` | `None` |
| `Vec<T>` | `[]` (vacío) |
| `String` | `""` (vacío) |
| `bool` | `false` |
| `u16`, `i32`, etc. | `0` |

### Default con valor custom

```rust
#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default = "default_port")]
    port: u16,
}

fn default_port() -> u16 {
    5432
}
```

## 5.2 `#[serde(rename = "...")]`

Cuando el nombre en el YAML/JSON no coincide con el nombre de tu campo:

```yaml
# YAML dice "db-name" con guión
db-name: mi_base
```

```rust
#[derive(Debug, Deserialize)]
struct Config {
    #[serde(rename = "db-name")]
    db_name: String,  // en Rust usás snake_case
}
```

### Renombrar todos los campos

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct Service {
    container_name: String,
    // convierte automáticamente camelCase → snake_case
}
```

Opciones: `snake_case`, `camelCase`, `PascalCase`, `SCREAMING_SNAKE_CASE`, `kebab-case`.

## 5.3 `#[serde(skip)]`

Ignorar un campo durante serialización/deserialización:

```rust
#[derive(Debug, Deserialize)]
struct Service {
    image: String,
    
    #[serde(skip)]
    internal_score: i32,  // no viene del YAML, lo calculamos nosotros
}
```

## 5.4 `#[serde(flatten)]`

"Desaplanar" un struct anidado al mismo nivel:

```yaml
# YAML
name: db
host: localhost
port: 5432
```

```rust
#[derive(Debug, Deserialize)]
struct Service {
    name: String,
    #[serde(flatten)]
    connection: ConnectionInfo,
}

#[derive(Debug, Deserialize)]
struct ConnectionInfo {
    host: String,
    port: u16,
}
```

Sin `flatten`, necesitarías que el YAML tenga `connection: { host: ..., port: ... }`. Con `flatten`, los campos `host` y `port` se mapean directo al struct `ConnectionInfo`.

---

# 6. Caso Real: Environment Variables del Compose

## 6.1 Formato Mapa vs Formato Lista

En Docker-Lens, el campo `environment` de un servicio puede venir en dos formatos. Ambos son válidos según la especificación de Docker Compose:

```yaml
# Formato A: Mapa
environment:
  POSTGRES_DB: mi_base
  POSTGRES_USER: admin
  POSTGRES_PASSWORD: secret123

# Formato B: Lista
environment:
  - POSTGRES_DB=mi_base
  - POSTGRES_USER=admin
  - POSTGRES_PASSWORD=secret123
```

Por eso en el struct usamos `serde_yaml::Value` en vez de un tipo fijo:

```rust
pub struct Service {
    pub environment: Option<serde_yaml::Value>,  // puede ser mapa O lista
}
```

## 6.2 Patrón: dos capas de desestructuración

Cuando sacás datos de un `Value`, siempre hay DOS capas de "abrir caja":

```
Capa 1: ¿el Value es un mapa o una lista?
  └─ as_mapping() → Some(map) o None
  └─ as_sequence() → Some(lista) o None

Capa 2: ¿el valor adentro es un string?
  └─ map.get("clave") → Option<&Value>
  └─ value.as_str() → Option<&str>
```

### Ejemplo con Mapa

```rust
if let Some(env) = &service.environment {           // Capa 0: abrir Option<Service.environment>
    if let Some(map) = env.as_mapping() {            // Capa 1: ¿es un mapa?
        if let Some(val) = map.get(                  // Capa 2a: buscar la clave
            &serde_yaml::Value::String("POSTGRES_DB".into())
        ) {
            if let Some(db_str) = val.as_str() {     // Capa 2b: extraer el string
                println!("DB: {}", db_str);
            }
        }
    }
}
```

### Ejemplo con Lista

```rust
if let Some(env) = &service.environment {
    if let Some(seq) = env.as_sequence() {           // Capa 1: ¿es una lista?
        for item in seq {                             // iterar cada elemento
            if let Some(s) = item.as_str() {         // Capa 2: es un string?
                if s.starts_with("POSTGRES_DB=") {
                    let value = s.trim_start_matches("POSTGRES_DB=");
                    println!("DB: {}", value);
                }
            }
        }
    }
}
```

## 6.3 Código completo de extracción

Así se vería en Docker-Lens, manejando ambos formatos:

```rust
fn extract_env_var(service: &Service, key: &str) -> Option<String> {
    let env = service.environment.as_ref()?;

    // Intentar como MAPA
    if let Some(map) = env.as_mapping() {
        let clave = serde_yaml::Value::String(key.into());
        if let Some(val) = map.get(&clave) {
            if let Some(s) = val.as_str() {
                return Some(s.to_string());
            }
        }
    }

    // Intentar como LISTA
    if let Some(seq) = env.as_sequence() {
        let prefix = format!("{}=", key);
        for item in seq {
            if let Some(s) = item.as_str() {
                if s.starts_with(&prefix) {
                    return Some(s[prefix.len()..].to_string());
                }
            }
        }
    }

    None
}

// Uso:
let db = extract_env_var(&service, "POSTGRES_DB");
let user = extract_env_var(&service, "POSTGRES_USER");
let password = extract_env_var(&service, "POSTGRES_PASSWORD");
```

¿Ves cómo el patrón se repite? **Una función que sabe abrir las dos capas, y la reusás para cada variable.**

---

# 7. Tips y Gotchas

## Tip 1: `serde_yaml::Value` no imprime lindo con `{}`
Usá `{:?}` (Debug) para ver el contenido:
```rust
println!("env: {:?}", service.environment);  // ✅
println!("env: {}", service.environment);     // ❌ no implementa Display
```

## Tip 2: Claves del Mapping son Values, no strings
Cuando buscás en un `BTreeMap<Value, Value>`, la clave es un `Value`, no un `&str`:
```rust
// ❌ NO compila
map.get("POSTGRES_DB")

// ✅ Correcto
map.get(&serde_yaml::Value::String("POSTGRES_DB".into()))
```

## Tip 3: `into()` convierte &str a Value
`"texto".into()` es azúcar sintáctico para crear un `Value::String("texto".to_string())`. Funciona porque Serde implementa `From<&str> for Value`.

## Tip 4: `as_str()` devuelve `&str`, no `String`
Si necesitás un `String` (porque lo vas a guardar en un struct):
```rust
if let Some(s) = val.as_str() {
    let owned = s.to_string();  // convertir &str → String
}
```

## Tip 5: Los campos del struct no necesitan estar en el mismo orden que el YAML
```yaml
# YAML
image: postgres:16
ports:
  - "5432:5432"
container_name: mi-db
```

```rust
// El orden en el struct NO importa
struct Service {
    container_name: Option<String>,  // está primero acá
    image: Option<String>,           // pero segundo en el YAML
    ports: Option<Vec<String>>,
}
```
Serde mapea por **nombre**, no por posición.

## Tip 6: `Option<T>` en Serde es "campo opcional" por defecto
No necesitás `#[serde(default)]` si el campo ya es `Option<T>`. Serde lo trata automáticamente como opcional.

## Tip 7: Errores de deserialización son descriptivos
```rust
match serde_yaml::from_str::<DockerCompose>(yaml_text) {
    Ok(compose) => { /* ... */ },
    Err(e) => {
        eprintln!("Error parseando YAML: {}", e);
        // Ejemplo de output:
        // "services.db.ports[0]: invalid type: integer `5432`, expected a string"
    }
}
```

Leé el error. Te dice exactamente qué campo falló y por qué.

## Tip 8: Podés parsear sin structs (modo rápido)
Si solo necesitás algo puntual y no querés modelar todo:
```rust
let value: serde_yaml::Value = serde_yaml::from_str(yaml_text).unwrap();
// value es un Value genérico, navegás con as_mapping(), etc.
```
Es más flexible pero perdés el tipado. Para scripts rápidos está OK. Para un proyecto, preferí structs.

## Tip 9: Serializar (escribir) también se puede
```rust
let compose = DockerCompose { /* ... */ };
let yaml_output = serde_yaml::to_string(&compose).unwrap();
println!("{}", yaml_output);
```
Útil para generar configs, debug, o exportar datos.

---

# 8. Resumen Rápido

### Conceptos clave

| Concepto | Qué es |
|----------|--------|
| **Serializar** | Struct → Texto (YAML/JSON/etc.) |
| **Deserializar** | Texto → Struct |
| **`#[derive(Deserialize)]`** | Genera automáticamente el código para deserializar |
| **`Option<T>`** | Campo que puede no existir en el input |
| **`serde_yaml::Value`** | Tipo genérico para cuando no sabés el formato exacto |
| **`.as_mapping()`** | ¿Este Value es un mapa? |
| **`.as_str()`** | ¿Este Value es un string? |
| **`.as_sequence()`** | ¿Este Value es una lista? |

### Patrón en Docker-Lens

```
YAML file
  └─ serde_yaml::from_str() → DockerCompose
       └─ .services → HashMap<String, Service>
            └─ service.environment → Option<Value>
                 └─ .as_mapping() o .as_sequence() → datos reales
```

### La regla de oro

> **Si sabés la estructura → usá structs con `#[derive(Deserialize)]`.**
> **Si NO sabés la estructura → usá `serde_yaml::Value` y navegás manualmente.**

En Docker-Lens, combinamos ambos: structs para la estructura general del compose, y `Value` para los campos que pueden variar de formato (environment, depends_on).

---

# Apéndice: Ejemplo Completo Docker-Lens

Todo junto, como referencia:

```rust
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

// === Structs del Compose ===

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
    pub depends_on: Option<serde_yaml::Value>,
}

// === Datos extraídos de la DB ===

#[derive(Debug)]
pub struct DBData {
    pub port: String,
    pub postgres_user: String,
    pub postgres_password: String,
    pub postgres_db: String,
}

// === Parseo ===

fn parse_compose(path: &PathBuf) -> Result<DockerCompose, std::io::Error> {
    let text = fs::read_to_string(path)?;
    serde_yaml::from_str(&text)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

// === Extracción de env vars ===

fn extract_env(service: &Service, key: &str) -> Option<String> {
    let env = service.environment.as_ref()?;

    // Intentar como mapa
    if let Some(map) = env.as_mapping() {
        let k = serde_yaml::Value::String(key.into());
        if let Some(val) = map.get(&k) {
            if let Some(s) = val.as_str() {
                return Some(s.to_string());
            }
        }
    }

    // Intentar como lista
    if let Some(seq) = env.as_sequence() {
        let prefix = format!("{}=", key);
        for item in seq {
            if let Some(s) = item.as_str() {
                if s.starts_with(&prefix) {
                    return Some(s[prefix.len()..].to_string());
                }
            }
        }
    }

    None
}

// === Extracción de puerto ===

fn extract_port(service: &Service) -> Option<String> {
    let ports = service.ports.as_ref()?;
    let raw = ports.first()?;
    let port = raw.split(':').last()?;
    Some(port.to_string())
}

// === Uso completo ===

fn build_db_data(service: &Service) -> Option<DBData> {
    Some(DBData {
        port: extract_port(service).unwrap_or_else(|| "5432".to_string()),
        postgres_user: extract_env(service, "POSTGRES_USER")?,
        postgres_password: extract_env(service, "POSTGRES_PASSWORD")?,
        postgres_db: extract_env(service, "POSTGRES_DB")?,
    })
}
```

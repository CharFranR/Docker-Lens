# Aprende Rust — Guía Completa para Docker-Lens

> Esta guía te lleva desde cero hasta poder contribuir comfortably a proyectos Rust reales. Está diseñada para alguien que viene de otros lenguajes (Python, JavaScript, Go, etc.) y quiere entender Rust de verdad — no solo "sintaxis", sino el modelo mental que hace que el compilador sea tu aliado.

---

## Índice / Temario

1. **Fundamentos del Lenguaje**
   - 1.1 Ownership (propiedad): el concepto más importante
   - 1.2 Borrowing (préstamos): referencias y mutabilidad
   - 1.3 Strings: String vs &str
   - 1.4 Funciones y firmas: Ownership en parámetros

2. **Manejo de Valores y Errores**
   - 2.1 Option\<T\>: el reemplazo de null
   - 2.2 Result\<T, E\>: errores como tipos
   - 2.3 Manejo de errores en práctica (para CLIs)

3. **Estructuras de Datos del Lenguaje**
   - 3.1 Structs: modelar entidades
   - 3.2 Enums: tipos suma y estados
   - 3.3 Tuplas y Newtypes
   - 3.4 Pattern matching con match

4. **Colecciones Estándar**
   - 4.1 Vec\<T\>: la lista por defecto
   - 4.2 HashMap y HashSet
   - 4.3 BTreeMap/BTreeSet: colecciones ordenadas
   - 4.4 Iteradores: iter(), iter_mut(), into_iter()

5. **Conceptos Avanzados (Introducción)**
   - 5.1 Lifetimes: referencias que viven lo necesario
   - 5.2 Módulos y crates: organizar código

6. **Arquitectura de Proyectos**
   - 6.1 lib + bin: la estructura recomendada
   - 6.2 Hexagonal / Ports & Adapters
   - 6.3 Functional core, imperative shell
   - 6.4 Capas y feature folders

7. **Herramientas del ecosistema CLI**
   - 7.1 Cargo: tu herramienta de trabajo
   - 7.2 clap: parsing de argumentos
   - 7.3 serde: parseo de YAML/JSON/TOML
   - 7.4 Logging: log + env_logger o tracing
   - 7.5 Errores con anyhow o thiserror

8. **Testing y Calidad**
   - 8.1 Tests unitarios e integración
   - 8.2 cargo fmt y clippy

9. **Próximos Pasos**

---

# 1. Fundamentos del Lenguaje

## 1.1 Ownership (propiedad) — el fundamento

### Por qué existe este concepto

Rust te protege de dos clases de bugs muy comunes en otros lenguajes:

1. **Use-after-free / double-free**: accedés a memoria que ya fue liberada, o la liberás dos veces.
2. **Data races**: dos threads modifican lo mismo al mismo tiempo sin coordinación.

Estos bugs son difíciles de detectar en testing y causan vulnerabilidades serias. Rust los evita con reglas en tiempo de compilación mediante dos mecanismos:

- **Ownership** (propiedad): quién "posee" un valor.
- **Borrowing** (préstamo): quién lo puede mirar o mutar, y por cuánto tiempo.

Si entendés esto, el 80% de Rust deja de ser magia.

### El compilador es tu par senior

No es broma: el compilador te fuerza a modelar bien el flujo de datos. Si te duele, es porque venís de lenguajes que te dejan hacer cualquier cosa y te enterás en runtime.

### Qué significa "ownership"

En Rust, **cada valor tiene un dueño** (una variable).

- Cuando el dueño sale de scope, el valor se libera automáticamente (**RAII**).
- Algunos tipos se **copian** (Copy), otros se **mueven** (Move).

Ejemplo (Move con `String`):

```rust
fn main() {
    let a = String::from("hola");
    let b = a; // MOVE: b ahora es el dueño

    // println!("{a}"); // ERROR: a fue movido
    println!("{b}");
}
```

Ejemplo (Copy con `i32`):

```rust
fn main() {
    let a: i32 = 10;
    let b = a; // COPY
    println!("a={a}, b={b}");
}
```

**Regla práctica**:

- Tipos "chiquitos" (números, `bool`, `char`, `&T`) suelen ser **Copy**.
- Tipos con heap (`String`, `Vec<T>`, `HashMap<K,V>`) suelen ser **Move**.

### Clonar no es copiar

Si necesitás duplicar datos del heap:

```rust
fn main() {
    let a = String::from("hola");
    let b = a.clone();
    println!("a={a}, b={b}");
}
```

**Tradeoff**:

- `clone()` **cuesta** (aloca y copia).
- Preferí **borrows** (`&str`, `&String`, `&[T]`) cuando puedas.

---

## 1.2 Borrowing (préstamos) — mirar vs mutar

### Préstamo inmutable: `&T`

```rust
fn len(s: &String) -> usize {
    s.len()
}

fn main() {
    let s = String::from("docker-lens");
    let n = len(&s);
    println!("{s} tiene {n} chars");
}
```

`s` **sigue siendo dueño**; la función solo "mira".

### Préstamo mutable: `&mut T`

```rust
fn shout(s: &mut String) {
    s.push('!');
}

fn main() {
    let mut s = String::from("hola");
    shout(&mut s);
    println!("{s}");
}
```

### La regla de oro (BORROW CHECKER)

En un mismo scope, Rust permite:

- **muchos** `&T` (lecturas), o
- **un solo** `&mut T` (una mutación)

Pero **NO** ambos al mismo tiempo.

Esto evita bugs clásicos: leer mientras alguien modifica.

---

## 1.3 Strings: `String` vs `&str` (donde muchos se rompen)

### Qué es cada uno

- `String`: dueño de un buffer en heap (mutable, crece).
- `&str`: referencia a una porción de texto (no dueño).

**Regla práctica**:

- APIs que *consumen* o *construyen*: suelen usar `String`.
- APIs que *leen*: preferí `&str`.

Ejemplo:

```rust
fn greet(name: &str) {
    println!("hola, {name}");
}

fn main() {
    let s = String::from("Franco");
    greet(&s);      // &String -> &str (deref coercion)
    greet("Loco"); // literal: &str
}
```

---

## 1.4 Estructura de funciones (firma, ownership en parámetros)

Tus parámetros dicen TODO sobre el contrato:

### Consumir (tomar ownership)

```rust
fn takes(s: String) {
    println!("{s}");
}
```

Usalo cuando:

- querés guardar el valor
- querés moverlo a otro dueño
- querés asegurar que nadie lo usa después

### Pedir prestado (leer)

```rust
fn reads(s: &str) -> usize {
    s.len()
}
```

### Pedir prestado mutable (mutar)

```rust
fn appends(s: &mut String) {
    s.push_str("!");
}
```

Sí, `&mut` es un contrato FUERTE: "nadie más toca esto mientras tanto".

### Retornos

- Podés devolver un valor nuevo (ownership al caller).
- Podés devolver `Result<T, E>` para fallas.

---

# 2. Manejo de Valores y Errores

## 2.1 Option\<T\>: puede o no estar

```rust
fn maybe_port() -> Option<u16> {
    Some(5432)
}

fn main() {
    match maybe_port() {
        Some(p) => println!("puerto={p}"),
        None => println!("sin puerto"),
    }
}
```

Patrones comunes:

```rust
if let Some(p) = maybe_port() {
    println!("{p}");
}

let p = maybe_port().unwrap_or(5432);
```

**EVITÁ** `unwrap()` en producción (sirve en protos/tests).

---

## 2.2 Result\<T, E\>: ok o error

```rust
fn parse_port(s: &str) -> Result<u16, std::num::ParseIntError> {
    s.parse::<u16>()
}

fn main() {
    match parse_port("5432") {
        Ok(p) => println!("ok {p}"),
        Err(e) => eprintln!("error: {e}"),
    }
}
```

El operador `?` (propaga errores) es CLAVE:

```rust
fn do_stuff() -> Result<(), Box<dyn std::error::Error>> {
    let p: u16 = "5432".parse()?; // si falla, retorna Err automáticamente
    println!("{p}");
    Ok(())
}
```

---

## 2.3 Manejo de errores "bien" (para un CLI tipo Docker-Lens)

En CLI, lo típico es:

1) funciones internas devuelven `Result<T, E>`
2) `main()` o el entrypoint imprime un mensaje humano y setea exit code

### Dos estrategias comunes

**A) Error propio (estricto, "arquitectura")**

- Pro: errores tipados, mantenible.
- Contra: más boilerplate al principio.

**B) `anyhow` (rápido, "producto")**

- Pro: cero fricción, buen contexto.
- Contra: perdés tipado fino.

Para aprender Rust, A es excelente; para avanzar MVP rápido, B es pragmático.

Ejemplo simple de error propio:

```rust
#[derive(Debug)]
enum AppError {
    Io(std::io::Error),
    ParsePort(std::num::ParseIntError),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(e: std::num::ParseIntError) -> Self { Self::ParsePort(e) }
}

fn read_and_parse() -> Result<u16, AppError> {
    let s = std::fs::read_to_string("port.txt")?;
    let p: u16 = s.trim().parse()?;
    Ok(p)
}
```

---

# 3. Estructuras de Datos del Lenguaje

## 3.1 Structs: modelar entidades

### Struct básico

```rust
#[derive(Debug)]
struct Service {
    name: String,
    image: Option<String>,
}
```

**Cuándo conviene**:

- modelo de dominio (Service, Project, DbConnectionInfo)
- agrupar datos relacionados

**Tradeoffs**:

- por defecto es *producto* (todos los campos juntos)
- no expresa "una cosa u otra" (para eso `enum`)

### impl (métodos y constructores)

```rust
impl Service {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), image: None, ports: vec![] }
    }

    fn is_postgres(&self) -> bool {
        self.name.to_lowercase().contains("postgres")
            || self.image.as_deref().unwrap_or("").contains("postgres")
    }
}
```

Regla práctica:

- lógica "del objeto" → método (`&self` / `&mut self`)
- lógica que usa muchas entidades → función libre en módulo (más testable a veces)

---

## 3.2 Enums: tipos suma y estados

### Enum básico

```rust
#[derive(Debug)]
enum DbKind {
    Postgres,
    Mysql,
    Sqlite,
    Unknown,
}
```

**Cuándo conviene**:

- estados, tipos, "shape" variable
- errores tipados (`enum AppError { Io(...), Parse(...), ... }`)
- reemplazo de "stringly typed" (menos `String` mágicas)

**Por qué es potente**:

- `match` te fuerza a cubrir todos los casos (o usar `_`)

### Ejemplo con valores

```rust
#[derive(Debug)]
enum ComposeValue {
    Str(String),
    Num(i64),
    Bool(bool),
    List(Vec<ComposeValue>),
    Map(std::collections::BTreeMap<String, ComposeValue>),
}
```

---

## 3.3 Tuplas y Newtypes

### Tuplas `(T1, T2, ...)` (rápidas, sin nombre)

```rust
let scored: (String, i32) = ("db".to_string(), 42);
```

**Cuándo conviene**:

- retornos chicos y locales (ej: `(host, port)`)
- usar con iteradores (ej: `Vec<(String, i32)>`)

**Cuándo NO**:

- si vas a pasar eso por muchos lados: ahí usá `struct` (te ahorrás confusión).

### Tuple struct y Newtype (envolver un tipo)

Esto es MUY Rust: agarrás un tipo base y le das un significado.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Port(u16);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ServiceName(String);
```

**Cuándo conviene**:

- evitar mezclar cosas que son el mismo tipo pero distinto significado (`u16` puerto vs `u16` otra cosa)
- hacer APIs más seguras

**Tradeoff**:

- más boilerplate (constructores, conversiones)

---

## 3.4 Pattern matching con match

```rust
fn classify(image: Option<&str>) -> DbKind {
    match image {
        Some(img) if img.contains("postgres") => DbKind::Postgres,
        Some(img) if img.contains("mysql") => DbKind::Mysql,
        _ => DbKind::Unknown,
    }
}
```

---

# 4. Colecciones Estándar

## 4.1 Vec\<T\>: la lista por defecto

**Qué es**: array dinámico en heap, contiguo.

**Cuándo conviene**:

- tu colección es "una lista"
- recorrés mucho, accedés por índice
- querés performance cache-friendly

**Operaciones**:

- push/pop al final: barato
- insertar al medio: caro (mueve elementos)

Ejemplo:

```rust
let mut ports = Vec::new();
ports.push(5432);
```

### VecDeque\<T\> (cola doble)

**Cuándo conviene**:

- necesitás push/pop adelante y atrás
- tipo "queue"/"buffer"

```rust
use std::collections::VecDeque;
let mut q = VecDeque::new();
q.push_back(1);
q.push_front(0);
```

---

## 4.2 HashMap y HashSet

### HashMap\<K, V\>

**Cuándo conviene**:

- acceso por clave, no necesitás orden
- lookup frecuente

**Cosas a saber**:

- no hay orden garantizado
- K debe implementar `Eq + Hash`

```rust
use std::collections::HashMap;

let mut m = HashMap::new();
m.insert("db".to_string(), 5432);

if let Some(p) = m.get("db") {
    println!("{p}");
}
```

**Patrón clave**: `entry()`

```rust
use std::collections::HashMap;

let mut freq: HashMap<String, i32> = HashMap::new();
let key = "postgres".to_string();
*freq.entry(key).or_insert(0) += 1;
```

### HashSet\<T\>

**Cuándo conviene**:

- querés "está o no está" sin duplicados
- whitelist/blacklist

```rust
use std::collections::HashSet;
let mut s = HashSet::new();
s.insert("postgres".to_string());
assert!(s.contains("postgres"));
```

---

## 4.3 BTreeMap/BTreeSet: colecciones ordenadas

**Cuándo conviene**:

- querés orden por clave (salida estable)
- rangos (keys entre A y B)

Tradeoff:

- un poco más lento que HashMap en promedio
- pero determinístico (útil en CLIs y tests)

```rust
use std::collections::BTreeMap;
let mut m = BTreeMap::new();
m.insert("a", 1);
m.insert("b", 2);
```

---

## 4.4 Iteradores: iter(), iter_mut(), into_iter()

### Tres formas de iterar

Supongamos un `Vec<String>`:

```rust
let v = vec![String::from("a"), String::from("b")];
```

- `v.iter()` → `&String` (borrow, NO mueve)
- `v.iter_mut()` → `&mut String` (borrow mutable, NO mueve)
- `v.into_iter()` → `String` (MUEVE, consume `v`)

Ejemplo:

```rust
for s in v.iter() {
    // s: &String
    println!("{s}");
}

// v todavía existe acá
```

En cambio:

```rust
for s in v.into_iter() {
    // s: String
    println!("{s}");
}

// v ya NO existe: fue consumido
```

Esto se conecta DIRECTO con el problema típico de HashMap y scoring.

### Ejemplo "Docker-Lens style": scoring sin mover el mapa

Idea: iterás por referencia, calculás score por servicio y guardás el resultado.

```rust
use std::collections::HashMap;

fn score_services(services: &HashMap<String, String>) -> HashMap<String, i32> {
    let mut scores = HashMap::new();

    for (name, image) in services.iter() {
        let mut score = 0;
        if name.contains("postgres") { score += 10; }
        if image.contains("postgres") { score += 20; }
        scores.insert(name.clone(), score); // acá clonás la key (porque scores debe ser dueño)
    }

    scores
}
```

Tradeoff:

- `name.clone()` cuesta, pero el mapa de salida necesita ser dueño.
- Alternativa más avanzada: devolver referencias (`&str`)… pero ahí entrás en lifetimes (ver sección siguiente).

---

# 5. Conceptos Avanzados (Introducción)

## 5.1 Lifetimes: referencias que viven lo necesario

Si estás empezando, quedate con esto:

> Lifetime = una etiqueta que le dice al compilador "esta referencia vive al menos tanto como…"

Cuando devolvés referencias, Rust necesita asegurar que no devolvés algo que se va a destruir.

Ejemplo clásico:

```rust
fn first<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() < b.len() { a } else { b }
}
```

**Consejo práctico para MVPs**:

- Si la vida de referencias te complica, **devolvé `String`** (owned) y listo.
- Pagás un costo de clone/alloc, pero ganás simplicidad.

---

## 5.2 Módulos y crates: organizar código

Reglas simples:

- `src/main.rs` → bin (CLI)
- `src/lib.rs` → library (reutilizable)
- `mod foo;` + `foo.rs` o `foo/mod.rs` para modularizar

Por qué importa:

- te ayuda a separar "dominio" de "infra"
- baja el acople y hace testear más fácil

---

# 6. Arquitectura de Proyectos

## 6.1 lib + bin: la estructura recomendada

### Qué es

- `src/lib.rs`: lógica reusable (parseo, scoring, queries)
- `src/main.rs`: CLI (parse args, imprimir, exit codes)

**Por qué es buena**:

- testeás el core sin depender de CLI
- menos acople, más claridad

Estructura sugerida:

```
src/
  lib.rs
  main.rs
  domain/
    mod.rs
  services/
    mod.rs
  adapters/
    mod.rs
  cli/
    mod.rs
```

---

## 6.2 Hexagonal / Ports & Adapters (muy buena para herramientas tipo Docker-Lens)

### Idea

Separás "qué querés hacer" (casos de uso) de "cómo lo hacés" (I/O real).

- **Dominio / Use-cases**: no saben nada de filesystem, Docker, Postgres driver
- **Ports (traits)**: interfaces
- **Adapters**: implementaciones concretas

### Ejemplo conceptual

```rust
// port
trait ComposeReader {
    fn read_compose(&self, path: &std::path::Path) -> Result<String, AppError>;
}

// use-case
fn detect_postgres(reader: &dyn ComposeReader, path: &std::path::Path) -> Result<Detection, AppError> {
    let yaml = reader.read_compose(path)?;
    // parse + score (lógica pura)
    Ok(Detection { /* ... */ })
}
```

**Pro**:

- test fácil (mock del trait)
- core puro, adaptable

**Contra**:

- más abstracción (para principiantes puede sentirse "mucho")

---

## 6.3 Functional core, imperative shell

### Idea

- **Core**: funciones puras que transforman datos (`input -> output`)
- **Shell**: I/O, logs, CLI, parse args

Ejemplo:

```rust
// core: puro
fn score_services(services: &[Service]) -> Vec<ScoredService> { /* ... */ }

// shell: I/O
fn run() -> Result<(), AppError> {
    let compose = std::fs::read_to_string("docker-compose.yml")?;
    let services = parse_compose(&compose)?;
    let scored = score_services(&services);
    print_report(&scored);
    Ok(())
}
```

**Pro**:

- mínimo acople
- tests muy simples

**Contra**:

- necesitás disciplina para no meter I/O en el core

---

## 6.4 Arquitectura por capas (Layered)

Capas típicas:

1. `cli/` (input/output)
2. `app/` (casos de uso)
3. `infra/` (fs, docker, db drivers)
4. `domain/` (tipos puros)

**Pro**: simple de entender

**Contra**: a veces se vuelve "capas por capas" sin necesidad (boilerplate)

---

## 6.5 Arquitectura por features (feature folders)

Ejemplo para Docker-Lens:

```
src/
  features/
    detect/
      mod.rs
      types.rs
      score.rs
      tests.rs
    inspect/
      mod.rs
      pg.rs
    export/
      mod.rs
      csv.rs
  shared/
    errors.rs
    fs.rs
  main.rs
  lib.rs
```

**Pro**:

- el código vive donde se usa
- escala bien cuando agregás subcomandos

**Contra**:

- si no tenés buen "shared", duplicás helpers

---

# 7. Herramientas del Ecosistema CLI

## 7.1 Cargo: tu herramienta de trabajo

Comandos que deberías usar todo el tiempo:

- `cargo run -- <args>`: ejecuta el bin
- `cargo test`: corre tests
- `cargo fmt`: formato
- `cargo clippy`: lint (te enseña idioms)
- `cargo doc --open`: docs locales

Perf/size (cuando te importe):

- `cargo build --release`

**Concepto clave**: en Rust, parte de "ser junior competente" es tener un workflow con `fmt+clippy+test`.

---

## 7.2 clap: parsing de argumentos

Si hacés CLIs, `clap` te resuelve:

- flags, options, subcommands
- help automático
- validación

Ejemplo mínimo (subcomandos):

```rust
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "docklens")]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Detect { #[arg(default_value = ".")] path: String },
    Inspect { service: String },
}

fn main() {
    let args = Args::parse();
    match args.cmd {
        Command::Detect { path } => println!("detect en {path}"),
        Command::Inspect { service } => println!("inspect {service}"),
    }
}
```

**Por qué es importante**: te fuerza a modelar el input como tipos (`struct Args`, `enum Command`). Eso ES Rust.

---

## 7.3 serde: parseo de YAML/JSON/TOML

Para CLIs que leen configs (como docker-compose), `serde` es central.

Conceptos:

- `#[derive(Serialize, Deserialize)]`
- structs que reflejan el formato
- `Option<T>` para campos opcionales

Ejemplo simple JSON:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    host: String,
    port: u16,
    verbose: Option<bool>,
}
```

Aprendizaje clave (vs Python): acá el parseo es tipado; si falla, falla temprano.

---

## 7.4 Logging: log + env_logger o tracing

Para CLIs "serias", se usa:

- `log` + `env_logger` (simple)
- o `tracing` + `tracing_subscriber` (más moderno)

Ejemplo con `log`:

```rust
use log::{debug, info, warn};

fn main() {
    env_logger::init();
    info!("arrancando");
    debug!("esto solo con RUST_LOG=debug");
    warn!("ojo con esto");
}
```

Por qué importa:

- te deja debuggear sin ensuciar stdout
- podés activar verbose con env var

---

## 7.5 Errores con anyhow o thiserror

Para CLIs, el usuario necesita "qué falló" + "dónde" + "qué hacer".

Dos estilos:

### anyhow (rápido)

- `anyhow::Result<T>`
- `context("...")`

### error propio + thiserror (limpio)

- `#[derive(thiserror::Error)]`
- mensajes claros por variante

**Regla práctica**:

- MVP: `anyhow`
- producto mantenible/biblioteca pública: error propio

---

## 7.6 Salidas y UX de CLI (stdout vs stderr)

Regla de oro:

- **stdout**: salida "consumible" (pipes, scripts)
- **stderr**: errores, logs, warnings

Ejemplo:

```rust
eprintln!("error: no se encontró docker-compose.yml");
std::process::exit(2);
```

Tip: si pensás en `jq`/Unix philosophy, tu CLI improve much más.

---

## 7.7 Exit codes (contrato con el sistema)

No todo es "0 o 1". Definí un mini contrato:

- `0`: OK
- `2`: input inválido / argumentos
- `3`: no encontrado
- `4`: fallo de conexión

Esto hace que tu herramienta sea usable en CI/scripts.

---

## 7.8 Leer archivos y paths (y no romper en Windows)

Aprendé a usar `std::path::Path` / `PathBuf`.

```rust
use std::path::Path;

fn read_file(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}
```

**Evitar**: concatenar strings tipo `format!("{}/foo", base)`.

**Preferir**:

```rust
use std::path::PathBuf;

let mut p = PathBuf::from(base);
p.push("docker-compose.yml");
```

---

## 7.9 Streams/IO grande (sin comerte la RAM)

Si exportás CSV o leés dumps:

- `read_to_string` te puede matar
- usá `BufRead` / `BufWriter`

Ejemplo:

```rust
use std::io::{self, BufRead};

fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.expect("stdin");
        println!("{line}");
    }
}
```

---

## 7.10 Concurrencia para CLIs: rayon vs tokio

Si venís de Python/JS: ojo con meter async por moda.

- `rayon`: paraleliza loops CPU/data-parallel (fácil)
- `tokio`: async I/O (muchas operaciones de red/DB concurrentes)

Regla de decisión:

- "Tengo una lista de 500 cosas y quiero procesarlas en paralelo" → `rayon`
- "Tengo miles de requests/consultas I/O" → `tokio`

---

## 7.11 "Idioms" que te hacen escribir Rust más limpio

- preferí `&str` sobre `&String` en APIs de lectura
- preferí `as_deref()` para pasar `Option<String>` a `Option<&str>`

```rust
let img: Option<String> = Some("postgres:16".into());
let is_pg = img.as_deref().is_some_and(|s| s.contains("postgres"));
```

- usá `map/and_then/filter` en Option/Result cuando haga el código más claro

---

# 8. Testing y Calidad

## 8.1 Tests unitarios e integración

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_port() {
        let p: u16 = "5432".parse().unwrap();
        assert_eq!(p, 5432);
    }
}
```

### Testing de CLIs (la diferencia entre hobby y herramienta)

Tres niveles:

1) Testear core como lib (unit tests) ✅
2) Tests de integración invocando el bin ✅
3) Snapshot tests para output estable (opcional)

Ejemplo integración (sin crates extra):

```rust
// tests/cli.rs
use std::process::Command;

#[test]
fn help_works() {
    let out = Command::new(env!("CARGO_BIN_EXE_docklens"))
        .arg("--help")
        .output()
        .expect("run");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("docklens"));
}
```

Punto clave: esto te obliga a mantener output consistente.

---

## 8.2 cargo fmt y clippy

Herramientas:

- `cargo test`
- `cargo fmt` (formato)
- `cargo clippy` (linter)

---

# 9. Ejercicios (hacelos, posta)

1) Escribí una función `normalize_image(image: &str) -> String` que:
   - pase a minúsculas
   - trim
   - reemplace múltiples espacios por uno (si no sabés cómo, hacé lo simple primero)

2) Modelá:
   - `Service { name: String, image: Option<String>, ports: Vec<String> }`
   - Una función `is_postgres(&Service) -> bool` sin usar `clone()`.

3) Implementá scoring con:
   - `HashMap<String, Service>` de input
   - `Vec<(String, i32)>` ordenado desc por score de output

---

# 10. Checklist mental (cuando el compilador te grita)

Cuando veas errores tipo "borrowed value does not live long enough", "use of moved value", etc., preguntate:

1) **Estoy moviendo** algo que después quiero usar?
2) Esta función debería tomar `T`, `&T` o `&mut T`?
3) Puedo resolverlo con:
   - pasar por referencia
   - reordenar scopes `{ ... }`
   - clonar (si está ok el costo)
4) Estoy mezclando `&T` y `&mut T` en el mismo scope?

---

# 11. Próximo paso (para TU proyecto)

Si querés, lo hacemos juntos y en tu código real:

- me decís cuál archivo te está costando más (`src/docker.rs`, `src/main.rs`, etc.)
- pegás el error exacto del compilador
- y lo traducimos a "qué regla de ownership estás violando"

Ahí es donde empezás a aprender de verdad.

---

# Apéndice: Resumen de estructuras de datos

### Cuándo usar cada cosa

| Necesitás | Usá |
|-----------|-----|
| Lista simple | `Vec<T>` |
| Cola (push/pop al frente) | `VecDeque<T>` |
| key → value (sin orden) | `HashMap<K, V>` |
| key → value (ordenado) | `BTreeMap<K, V>` |
| "está o no está" (sin orden) | `HashSet<T>` |
| "está o no está" (ordenado) | `BTreeSet<T>` |
| Top-N / priority queue | `BinaryHeap<T>` |

### Newtypes útiles

```rust
struct Port(u16);
struct ServiceName(String);
```

Evitan mezclar tipos que son técnicamente iguales pero significan cosas distintas.

---

# Guía de Ejercicios (10 ejercicios tuanis)

> Estos ejercicios van aumentando en dificultad gradual. Empezá por el 1 y no te salts ninguno hasta que te sientas cómodo. Cada uno refuerza conceptos clave de la guía.

## Ejercicio 1: Hola, Ownership

**Objetivo**: Entender Move vs Copy.

```rust
fn main() {
    // Ejercicio: ejecutá este código y observá los errores
    
    // a) Esto funciona?
    let a = 42;
    let b = a;
    println!("a={}, b={}", a, b);
    
    // b) Esto falla. Por qué?
    // let s1 = String::from("hola");
    // let s2 = s1;
    // println!("{}", s1);
    
    // c) Cómo lo corregís sin usar clone()?
}
```

**Tu tarea**: Descomentá el bloque b) y corregilo de dos formas:
1. Usando `clone()`
2. Sin usar `clone()` (solo con referencias)

---

## Ejercicio 2: Préstamos (Borrowing)

**Objetivo**: Dominar `&T` vs `&mut T`.

```rust
fn main() {
    let mut texto = String::from("Hola");
    
    // Ejercicio 2a: función que solo lee
    fn longitud(s: &String) -> usize {
        s.len()
    }
    println!("Longitud: {}", longitud(&texto));
    
    // Ejercicio 2b: función que modifica
    fn agregar_exclamacion(s: &mut String) {
        s.push('!');
    }
    agregar_exclamacion(&mut texto);
    println!("{}", texto);
    
    // Ejercicio 2c: este código falla, explicá por qué
    // let mut s = String::from("hola");
    // let r1 = &s;
    // let r2 = &mut s;  // ERROR!
    // println!("{} {} {}", s, r1, r2);
}
```

**Tu tarea**: Descomentá el bloque 2c, ejecutalo, leé el error y explicá en tus palabras qué dice el borrow checker.

---

## Ejercicio 3: Option y Result

**Objetivo**: Manejar valores que pueden o no existir, y errores como tipos.

```rust
// Dado este enum de errores:
#[derive(Debug)]
enum ApiError {
    NotFound,
    Unauthorized,
    ServerError(String),
}

// Y esta función que puede fallar:
fn get_user(id: u32) -> Result<Option<String>, ApiError> {
    match id {
        1 => Ok(Some(String::from("Franco"))),
        2 => Ok(None),  // usuario no existe
        _ => Err(ApiError::ServerError(format!("id {} no válido", id))),
    }
}

fn main() {
    // Ejercicio: usá match para manejar los 3 casos
    let resultado = get_user(1); // probá con 1, 2, y 999
    
    // Tu código aquí:
    // - Si Ok(Some(name)) => imprimir "Hola, {name}"
    // - Si Ok(None) => "Usuario no encontrado"
    // - Si Err(e) => "Error: {e:?}"
}
```

**Pista**: Usá `match` con los tres patrones posibles.

---

## Ejercicio 4: Structs y Methods

**Objetivo**: Modelar un dominio simple con métodos asociados.

```rust
#[derive(Debug)]
struct Service {
    name: String,
    image: Option<String>,
    ports: Vec<u16>,
}

impl Service {
    // Ejercicio 4a: constructor
    fn new(name: impl Into<String>) -> Self {
        // completar
    }
    
    // Ejercicio 4b: método que dice si es Postgres
    fn is_postgres(&self) -> bool {
        // devolver true si name o image contienen "postgres"
    }
    
    // Ejercicio 4c: agregar puerto si no existe
    fn add_port(&mut self, port: u16) {
        // agregar solo si no está
    }
}

fn main() {
    let mut svc = Service::new("db-postgres");
    println!("Es postgres? {}", svc.is_postgres());
    
    svc.add_port(5432);
    svc.add_port(5432); // no debería duplicar
    println!("Ports: {:?}", svc.ports);
}
```

**Tu tarea**: Completá los métodos. Probá con diferentes inputs.

---

## Ejercicio 5: Enum con Datos

**Objetivo**: Usar enums para modelar estados con información.

```rust
#[derive(Debug)]
enum ContainerState {
    Running,
    Stopped,
    Paused { reason: String },
    Failed { code: i32, message: String },
}

#[derive(Debug)]
struct Container {
    name: String,
    state: ContainerState,
}

fn status_label(c: &Container) -> String {
    // Ejercicio: usar match para devolver string descriptivo
    // - Running => "🟢 {name} ejecutándose"
    // - Stopped => "🔴 {name} detenido"
    // - Paused { reason } => "🟡 {name} pausado: {reason}"
    // - Failed { code, message } => "❌ {name} falló [{code}]: {message}"
}

fn main() {
    let containers = vec![
        Container { name: "postgres".into(), state: ContainerState::Running },
        Container { name: "redis".into(), state: ContainerState::Stopped },
        Container { name: "worker".into(), state: ContainerState::Paused { reason: "alta carga".into() } },
        Container { name: "broken".into(), state: ContainerState::Failed { code: 1, message: "OOM".into() } },
    ];
    
    for c in &containers {
        println!("{}", status_label(c));
    }
}
```

**Tu tarea**: Completá la función `status_label`.

---

## Ejercicio 6: Colecciones y Scoring

**Objetivo**: Usar HashMap y Vec para calcular scores.

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct DockerService {
    name: String,
    image: String,
    ports: Vec<u16>,
}

fn score_services(services: &[DockerService]) -> Vec<(String, i32)> {
    // Ejercicio: given un slice de servicios, calcular score:
    // - +10 puntos si name contiene "postgres"
    // - +20 puntos si image contiene "postgres"
    // - +5 puntos por cada puerto
    // Devolver Vec<(name, score)> ordenado por score DESC
    
    let mut scores: HashMap<String, i32> = HashMap::new();
    
    // Tu código aquí
    
    // Convertir a Vec y ordenar
    let mut result: Vec<(String, i32)> = scores.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1)); // descendente
    result
}

fn main() {
    let services = vec![
        DockerService { name: "db".into(), image: "postgres:16".into(), ports: vec![5432, 5433] },
        DockerService { name: "cache".into(), image: "redis:7".into(), ports: vec![6379] },
        DockerService { name: "postgres-backup".into(), image: "postgres:15".into(), ports: vec![] },
        DockerService { name: "app".into(), image: "node:20".into(), ports: vec![3000] },
    ];
    
    let scored = score_services(&services);
    println!("{:?}", scored);
    // Expected: [("db", 55), ("postgres-backup", 30), ("cache", 6), ("app", 5)]
}
```

**Tu tarea**: Completá la función. El resultado esperado está en el comentario.

---

## Ejercicio 7: Iteradores Avanzados

**Objetivo**: Dominar `iter()`, `iter_mut()`, `into_iter()`.

```rust
fn main() {
    let mut servicios = vec![
        String::from("postgres"),
        String::from("redis"),
        String::from("nginx"),
    ];
    
    // Ejercicio 7a: iter() - solo lectura
    println!("\n--- iter() ---");
    for s in servicios.iter() {
        println!("{}", s);  // s es &String
    }
    println!("Vec original: {:?}", servicios);
    
    // Ejercicio 7b: iter_mut() - modificar sin consumir
    println!("\n--- iter_mut() ---");
    for s in servicios.iter_mut() {
        s.push_str("-old");  // modifica cada elemento
    }
    println!("Vec modificado: {:?}", servicios);
    
    // Ejercicio 7c: into_iter() - consumir el vec
    println!("\n--- into_iter() ---");
    for s in servicios.into_iter() {
        println!("{}", s);  // s es String (dueño)
    }
    // println!("{:?}", servicios); // ERROR: servicios fue consumido
}
```

**Tu tarea**: Ejecutá el código y explicá qué cambia en cada caso.

---

## Ejercicio 8: Parser con Serde (JSON)

**Objetivo**: Parsear JSON a tipos de Rust.

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    name: String,
    version: String,
    #[serde(default)]
    env: Vec<EnvVar>,
}

#[derive(Debug, Deserialize)]
struct EnvVar {
    key: String,
    value: String,
}

fn main() {
    let json = r#"{
        "name": "docker-lens",
        "version": "0.1.0",
        "env": [
            { "key": "DEBUG", "value": "true" },
            { "key": "PORT", "value": "5432" }
        ]
    }"#;
    
    // Ejercicio: parsear el JSON a Config
    let config: Config = /* tu código aquí */;
    
    println!("{:?}", config);
}
```

**Tu tarea**: Completá el parseo usando `serde_json::from_str`.

---

## Ejercicio 9: Error Handling Completo

**Objetivo**: Crear un sistema de errores propio con `thiserror`.

```rust
use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("No se pudo leer el archivo {0}: {1}")]
    Io(#[source] std::io::Error),
    
    #[error("Puerto inválido: {0}")]
    InvalidPort(String),
    
    #[error("Archivo no encontrado: {0}")]
    FileNotFound(String),
}

fn parse_port_from_file(path: &Path) -> Result<u16, AppError> {
    // Ejercicio: leer archivo, parsear a u16, validar rango 1-65535
    // 1. fs::read_to_string(path) -> Result<String, std::io::Error>
    //    (usar ? para convertir a AppError::Io)
    // 2. trim() y parse::<u16>() -> Result<u16, ParseIntError>
    //    (usar ? para convertir a AppError::InvalidPort)
    // 3. validar rango y devolver
    
    // Tu código aquí
}

fn main() {
    // Probá con un archivo que contenga "5432"
    match parse_port_from_file(Path::new("port.txt")) {
        Ok(p) => println!("Puerto: {}", p),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

**Tu tarea**: Completá la función. Si no tenés el archivo, crealo.

---

## Ejercicio 10: Proyecto Completo (Mini CLI)

**Objetivo**: Unir todo en un mini proyecto.

```rust
// Creá un archivo src/main.rs que:
// 1. Use clap para parsear argumentos: `cargo run -- --file docker-compose.yml`
// 2. Lea el archivo (si no existe, error friendly)
// 3. Busque servicios que contengan "postgres" en name o image
// 4. Imprima: "Servicio: {name} | Image: {image}" para cada match

// Estructura sugerida:
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    file: String,
}

fn main() {
    let args = Args::parse();
    // Tu implementación
}

// Para probar, creá un docker-compose.yml de mentira:
// services:
//   db:
//     image: postgres:16
//   cache:
//     image: redis:7
```

**Tu tarea**: Implementá el mini CLI. Usá todo lo que aprendiste:
- `clap` para argumentos
- `std::fs` para leer archivos
- `serde` (opcional) o string parsing simple
- `Result` para errores
- `HashMap` o `Vec` para filtrar servicios

---

## Soluciones (solo mirá después de intentar)

Acá van las respuestas sugeridas. Si te atascaste, mirá una pista y volvé a intentar.

### Ejercicio 1
```rust
// 2a: con clone
let s1 = String::from("hola");
let s2 = s1.clone();
println!("{}", s1); // funciona

// 2b: con referencia
let s1 = String::from("hola");
let s2 = &s1;
println!("{}", s1); // funciona, s2 solo mira
```

### Ejercicio 2c
El error dice que no podés tener una referencia mutable mientras existen referencias inmutables. La regla de oro del borrow checker.

### Ejercicio 3
```rust
match resultado {
    Ok(Some(name)) => println!("Hola, {}", name),
    Ok(None) => println!("Usuario no encontrado"),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

### Ejercicio 4
```rust
fn new(name: impl Into<String>) -> Self {
    Self { name: name.into(), image: None, ports: vec![] }
}

fn is_postgres(&self) -> bool {
    self.name.to_lowercase().contains("postgres") 
        || self.image.as_deref().map_or(false, |i| i.contains("postgres"))
}

fn add_port(&mut self, port: u16) {
    if !self.ports.contains(&port) {
        self.ports.push(port);
    }
}
```

### Ejercicio 5
```rust
fn status_label(c: &Container) -> String {
    match &c.state {
        ContainerState::Running => format!("🟢 {} ejecutándose", c.name),
        ContainerState::Stopped => format!("🔴 {} detenido", c.name),
        ContainerState::Paused { reason } => format!("🟡 {} pausado: {}", c.name, reason),
        ContainerState::Failed { code, message } => format!("❌ {} falló [{}]: {}", c.name, code, message),
    }
}
```

### Ejercicio 6
```rust
fn score_services(services: &[DockerService]) -> Vec<(String, i32)> {
    let mut scores = HashMap::new();
    
    for svc in services {
        let mut score = 0;
        if svc.name.contains("postgres") { score += 10; }
        if svc.image.contains("postgres") { score += 20; }
        score += svc.ports.len() as i32 * 5;
        scores.insert(svc.name.clone(), score);
    }
    
    let mut result: Vec<(String, i32)> = scores.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}
```

### Ejercicio 7
Ver explicación en el código. La clave: `iter()` → referencia, `iter_mut()` → mutable pero no consume, `into_iter()` → consume.

### Ejercicio 8
```rust
let config: Config = serde_json::from_str(json).unwrap();
```

### Ejercicio 9
```rust
fn parse_port_from_file(path: &Path) -> Result<u16, AppError> {
    let content = fs::read_to_string(path).map_err(AppError::Io)?;
    let port: u16 = content.trim().parse().map_err(|e| AppError::InvalidPort(e.to_string()))?;
    
    if port == 0 || port > 65535 {
        return Err(AppError::InvalidPort(format!("{} fuera de rango", port)));
    }
    Ok(port)
}
```

### Ejercicio 10
¡Este es el ejercicio de integración! No hay solución única. Lo importante es que funcione y uses los conceptos de la guía.
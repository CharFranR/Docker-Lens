# Docker-Lens

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)
[![Rust 2024](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)

CLI para acceder a bases de datos en docker-compose sin configuración manual. Detecta el motor, extrae las credenciales y conecta automáticamente.

## Prerrequisitos

- Python 3.8+
- Docker corriendo
- Una de las DBs soportadas en tu `docker-compose.yml`

## Stack

| Capa | Tecnología |
|------|------------|
| Core | Rust 2024, PyO3 |
| Docker | bollard (API nativa) |
| CLI | Python 3.8+, Click |
| DBs | psql, mysql CLI, mongodb crate, rusqlite |

## Architecture

```
docker-lens
├── heuristic    → Detección de DB en docker-compose.yml (scoring ponderado)
├── db/
│   ├── docker   → Resolución de IP via bollard
│   ├── postgres → Adapter psql
│   ├── mysql    → Adapter mysql CLI
│   ├── mongo    → Adapter mongodb crate (async)
│   └── sqlite   → Adapter rusqlite (bundled)
└── cli          → Comandos Click (Python)
```

## Quick start

```bash
pip install docker-lens

# Info de la DB detectada
docker-lens info .

# Listar tablas
docker-lens tables .

# Query directa
docker-lens query "SELECT * FROM usuarios" .

# Exportar a CSV
docker-lens export-csv usuarios .

# Migrar a SQLite
docker-lens export-sqlite . -o backup.db
```

El argumento `.` es la ruta al directorio que contiene tu `docker-compose.yml`. Puede ser `.` (directorio actual) o cualquier otra ruta.

## Cómo funciona

Docker-Lens analiza tu `docker-compose.yml` y asigna puntaje a cada servicio según:
- **Imagen** (30 pts) — coincide con patrones conocidos del motor
- **Puerto** (25 pts) — expone el puerto por defecto de la DB
- **Variables de entorno** (15-20 pts) — tiene keys como `POSTGRES_USER`, `MYSQL_ROOT_PASSWORD`, etc.
- **Nombre del servicio** (10 pts) — se llama `db`, `postgres`, `mongo`, etc.

El servicio con mayor puntaje gana. Las credenciales se extraen de las variables de entorno del compose, con defaults razonables si faltan.

## Comandos

| Comando | Descripción |
|---------|-------------|
| `info` | Credenciales detectadas |
| `tables` | Lista tablas/collections |
| `query` | SQL o MongoDB queries |
| `head` / `tail` | Primeras / últimas N filas |
| `schema` | Columnas, tipos y constraints |
| `count` | Cantidad de filas |
| `export-csv` | Exporta una tabla a CSV |
| `export-all` | Exporta todas las tablas |
| `export-sqlite` | Migra la DB completa a SQLite |
| `shell` | Sesión interactiva (psql/mysql/mongosh) |
| `truncate` / `drop` | Vacía o elimina una tabla |

## DBs soportadas

PostgreSQL, MySQL, MariaDB, MongoDB, SQLite. La detección es automática mediante scoring ponderado sobre imagen, puerto y variables de entorno del compose.

## Desarrollo

```bash
# Build
maturin build --release

# Instalar localmente
pip install target/wheels/*.whl

# Tests
cargo test
```

## Licencia

MIT

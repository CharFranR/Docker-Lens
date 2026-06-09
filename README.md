# Docker-Lens

CLI para acceder a bases de datos en docker-compose sin configuración manual. Detecta el motor, extrae las credenciales y conecta automáticamente.

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

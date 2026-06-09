# Docker-Lens

Accede a tu base de datos desde el terminal. Docker-Lens detecta automáticamente qué DB tienes en tu `docker-compose.yml` y te conecta al instante.

## Instalación

```bash
pip install docker-lens
```

Requisitos: Python 3.8+, Docker corriendo, una de las DBs soportadas en tu compose.

## Uso

```bash
# Ver credenciales detectadas
docker-lens info .

# Listar tablas
docker-lens tables .

# Ejecutar una query
docker-lens query "SELECT * FROM usuarios" .

# Primeras / últimas filas
docker-lens head usuarios . -n 20
docker-lens tail usuarios . -n 10

# Schema de una tabla
docker-lens schema usuarios .

# Cantidad de filas
docker-lens count usuarios .

# Exportar a CSV
docker-lens export-csv usuarios .
docker-lens export-all . -o ./backups

# Migrar a SQLite
docker-lens export-sqlite . -o backup.db

# Sesión interactiva
docker-lens shell .
```

## Comandos

| Comando | Descripción |
|---------|-------------|
| `info` | Credenciales detectadas |
| `tables` | Lista tablas/collections |
| `query` | Ejecuta SQL o MongoDB queries |
| `head` | Primeras N filas |
| `tail` | Últimas N filas |
| `schema` | Columnas, tipos y constraints |
| `count` | Cantidad de filas |
| `export-csv` | Exporta una tabla a CSV |
| `export-all` | Exporta todas las tablas |
| `export-sqlite` | Migra la DB completa a SQLite |
| `connect` | Muestra el comando de conexión |
| `shell` | Abre sesión interactiva (psql/mysql/mongosh) |
| `truncate` | Vacía una tabla |
| `drop` | Elimina una tabla |

## DBs soportadas

PostgreSQL, MySQL, MariaDB, MongoDB y SQLite. La detección es automática — no hace falta pasar credenciales, puerto ni nombre del servicio.

## Desarrollo

El core está en Rust (PyO3 + bollard), la CLI en Python (Click).

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

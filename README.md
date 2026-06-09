---
layout: default
title: Docker-Lens
---

# Docker-Lens

**Accedé a tu base de datos desde el terminal. Sin configuración, sin complicaciones.**

Docker-Lens detecta automáticamente qué base de datos tenés en tu `docker-compose.yml` y te conecta al instante. PostgreSQL, MySQL, MariaDB, MongoDB, SQLite — un solo comando para todos.

---

## Instalación

```bash
pip install docker-lens
```

## Uso rápido

```bash
# Ver credenciales detectadas
docker-lens info .

# Listar tablas
docker-lens tables .

# Ejecutar una query
docker-lens query "SELECT * FROM usuarios" .

# Ver primeras filas
docker-lens head usuarios . -n 20

# Exportar a CSV
docker-lens export-csv usuarios .

# Exportar toda la base a SQLite
docker-lens export-sqlite . -o backup.db
```

---

## Comandos

| Comando | Descripción |
|---------|-------------|
| `info` | Muestra las credenciales detectadas |
| `tables` | Lista todas las tablas/collections |
| `query` | Ejecuta SQL o MongoDB queries |
| `head` | Primeras N filas de una tabla |
| `tail` | Últimas N filas |
| `schema` | Columnas, tipos y constraints |
| `count` | Cantidad de filas |
| `export-csv` | Exporta una tabla a CSV |
| `export-all` | Exporta todas las tablas |
| `export-sqlite` | Migra toda la DB a SQLite |
| `connect` | Muestra el comando de conexión |
| `shell` | Abre sesión interactiva |
| `truncate` | Vacía una tabla |
| `drop` | Elimina una tabla |

---

## Motor detectado automáticamente

Docker-Lens analiza tu `docker-compose.yml` y detecta:

- **PostgreSQL** — imagen `postgres`, `postgis`, `timescaledb`
- **MySQL** — imagen `mysql`, `bitnami/mysql`
- **MariaDB** — imagen `mariadb`, `bitnami/mariadb`
- **MongoDB** — imagen `mongo`, `bitnami/mongodb`
- **SQLite** — imagen `keinos/sqlite3`, `sqlite3`

No necesitás pasar credenciales. No necesitás saber el puerto. No necesitás saber el nombre del servicio. Docker-Lens lo resuelve solo.

---

## Stack

- **Rust** (PyO3) — detección, parsing, conexión a DBs
- **Python** (Click) — interfaz de comandos
- **bollard** — comunicación nativa con Docker daemon
- **rusqlite** — SQLite bundled sin dependencias del sistema
- **mongodb** — driver nativo para MongoDB

---

## Licencia

MIT

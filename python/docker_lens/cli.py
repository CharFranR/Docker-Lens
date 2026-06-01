import subprocess
import json

import docker_lens
import click
import os

@click.group()
def cli():
    """Docker-Lens: accedé a tu base de datos desde el terminal"""
    pass


@cli.command(help="Muestra las credenciales de la base de datos")
@click.argument("path", required=True)
def info(path):
    # docker-lens info [path]

    if path == ".":
        path = os.getcwd()

    try:
        orchestator_path = docker_lens.find_orchestrator_py(path)
    except RuntimeError:
        click.echo("No se encontró un archivo docker-compose.yml en este proyecto", err=True)
        return
    
    try:
        c = docker_lens.find_db_py(orchestator_path)
        db_type = c.get("db_type", "unknown")
        click.echo(f"DB Type:     {db_type}")
        click.echo(f"Host:        {c.get('host', 'N/A')}")
        click.echo(f"Port:        {c.get('port', 'N/A')}")
        click.echo(f"User:        {c.get('user', 'N/A')}")
        click.echo(f"Database:    {c.get('database', 'N/A')}")
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
  

@cli.command(help="Cuenta todas las tablas de la base de datos")
@click.argument("path", required=True)
def tables(path):
    # docker-lens tables [path]

    if path == ".":
        path = os.getcwd()

    try:
        orchestator_path = docker_lens.find_orchestrator_py(path)
    
    except RuntimeError:
        click.echo("No se encontró un archivo docker-compose.yml en este proyecto", err=True)
        return
    
    try:
        credenciales = docker_lens.find_db_py(orchestator_path)
        tablas = docker_lens.list_tables_py(credenciales)
        click.echo(tablas)

    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
    

@cli.command(help="Ejecuta una consulta SQL custom (o JSON para MongoDB)")
@click.argument("query", required = True)
@click.argument("path", required = True)

def query(query, path):
    # docker-lens query "<sql | json>" [path]
    
    if path == ".":
        path = os.getcwd()

    try:
        orchestator_path = docker_lens.find_orchestrator_py(path)
    
    except RuntimeError:
        click.echo("No se encontró un archivo docker-compose.yml en este proyecto", err=True)
        return
    
    try:
        credenciales = docker_lens.find_db_py(orchestator_path)
        db_type = credenciales.get("db_type", "postgres")

        # For MongoDB, if the query doesn't look like JSON, wrap it as a find
        actual_query = query
        if db_type == "mongo":
            actual_query = _mongo_query(query)

        response = docker_lens.make_query_py(credenciales, actual_query)
        click.echo(response)

    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)


def _mongo_query(raw: str) -> str:
    """Convert a user-friendly query to MongoDB JSON format.
    
    Formats:
      - If raw is valid JSON, return as-is
      - 'collection' → find all in collection
      - 'collection:{"field": "value"}' → find with filter
    """
    raw_stripped = raw.strip()
    # Already JSON
    if raw_stripped.startswith("{"):
        return raw_stripped

    # Format: collection:filter
    if ":" in raw_stripped:
        parts = raw_stripped.split(":", 1)
        coll = parts[0].strip()
        filter_str = parts[1].strip()
        try:
            filt = json.loads(filter_str)
        except json.JSONDecodeError:
            filt = {}
        return json.dumps({"find": coll, "filter": filt, "limit": 100})
    
    # Just collection name
    return json.dumps({"find": raw_stripped, "filter": {}, "limit": 100})


@cli.command(help="Muestra las primeras N filas de una tabla")
@click.argument("table_name")
@click.option("-n", "--limit", default=10, type=int, help="Cantidad de filas")
@click.argument("path", required=True)

def head(table_name, limit,  path):
    # docker-lens head <tabla> [n] [path]	Primeras N filas (default 10)
    
    if not limit:
        limit = 10
    
    if path == ".":
        path = os.getcwd()

    try:
        orchestator_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(orchestator_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    db_type = c.get("db_type", "postgres")

    if db_type == "mongo":
        q = json.dumps({"find": table_name, "filter": {}, "limit": limit})
        query.callback(q, path)
    else:
        sql = f"SELECT * FROM {table_name} LIMIT {limit};"
        query.callback(sql, path)


@cli.command(help="Muestra las últimas N filas de una tabla")
@click.argument("table_name")
@click.option("-n", "--limit", default=10, type=int, help="Cantidad de filas")
@click.argument("path", required=True)
def tail(table_name, limit,  path):
    # docker-lens tail <tabla> [n] [path]	Últimas N filas
    
    if not limit:
        limit = 10
    
    if path == ".":
        path = os.getcwd()

    try:
        orchestator_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(orchestator_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    db_type = c.get("db_type", "postgres")

    if db_type == "mongo":
        # MongoDB: aggregate with sort + limit
        q = json.dumps({
            "aggregate": table_name,
            "pipeline": [
                {"$sort": {"_id": -1}},
                {"$limit": limit}
            ]
        })
        query.callback(q, path)
    else:
        sql = f"SELECT * FROM {table_name} ORDER BY id DESC LIMIT {limit};"
        query.callback(sql, path)


@cli.command(help="Muestra las columnas, tipos y constraints de una tabla")
@click.argument("table_name")
@click.argument("path", required=True)
def schema(table_name, path):
    # docker-lens schema <tabla> [path]	Columnas, tipos, constraints
    
    if path == ".":
        path = os.getcwd()

    try:
        orchestator_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(orchestator_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    db_type = c.get("db_type", "postgres")

    if db_type == "mongo":
        # Sample one document to infer schema
        q = json.dumps({"find": table_name, "limit": 1})
        query.callback(q, path)
    elif db_type in ("mysql", "mariadb"):
        db_name = c.get("database", "")
        sql = (
            f"SELECT column_name, data_type, is_nullable, column_default "
            f"FROM information_schema.columns "
            f"WHERE table_name = '{table_name}' AND table_schema = '{db_name}' "
            f"ORDER BY ordinal_position;"
        )
        query.callback(sql, path)
    elif db_type == "sqlite":
        sql = f'PRAGMA table_info("{table_name}");'
        query.callback(sql, path)
    else:
        # PostgreSQL (default)
        sql = (
            f"SELECT column_name, data_type, is_nullable, column_default "
            f"FROM information_schema.columns "
            f"WHERE table_name = '{table_name}' "
            f"ORDER BY ordinal_position;"
        )
        query.callback(sql, path)


@cli.command(help="Cuenta cantidad de filas en una tabla")
@click.argument("table_name")
@click.argument("path", required=True)
def count(table_name, path):
    # docker-lens count <tabla> [path]	Cantidad de filas
    
    if path == ".":
        path = os.getcwd()

    try:
        orchestator_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(orchestator_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    db_type = c.get("db_type", "postgres")

    if db_type == "mongo":
        q = json.dumps({
            "aggregate": table_name,
            "pipeline": [{"$count": "total"}]
        })
        query.callback(q, path)
    else:
        sql = f"SELECT COUNT(*) FROM {table_name};"
        query.callback(sql, path)


@cli.command(help="Vacía una tabla (CASCADE si tiene dependencias)")
@click.argument("table_name")
@click.option("--force", is_flag=True, help="Saltar confirmacion")
@click.argument("path", required=True)
def truncate(table_name, force, path):
    # docker-lens truncate <tabla> [path]	Vacía la tabla
    
    if not force:
        click.confirm(f"¿Vaciar la tabla '{table_name}'? Esto es irreversible.", abort=True)

    if path == ".":
        path = os.getcwd()

    try:
        orchestator_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(orchestator_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    db_type = c.get("db_type", "postgres")

    if db_type == "mongo":
        q = json.dumps({"find": table_name, "filter": {}, "limit": 0})
        click.echo(f"Para MongoDB, usá: docker-lens query '{{\"find\": \"{table_name}\", \"filter\": {{}}}}' y eliminá manualmente")
        return

    sql = f"TRUNCATE TABLE {table_name} CASCADE;"
    query.callback(sql, path)


@cli.command(help="Elimina una tabla (CASCADE si tiene dependencias)")
@click.argument("table_name")
@click.option("--force", is_flag=True, help="Saltar confirmacion")
@click.argument("path", required=True)
def drop(table_name, force, path):
    # docker-lens drop <tabla> [path]	Elimina la tabla
    
    if not force:
        click.confirm(f"¿Vaciar la tabla '{table_name}'? Esto es irreversible.", abort=True)

    if path == ".":
        path = os.getcwd()

    try:
        orchestator_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(orchestator_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    db_type = c.get("db_type", "postgres")

    if db_type == "mongo":
        q = json.dumps({"find": table_name, "filter": {}, "limit": 0})
        click.echo(f"Para MongoDB, usá: docker-lens query '{{\"find\": \"{table_name}\", \"filter\": {{}}}}' y eliminá manualmente")
        return

    sql = f"DROP TABLE {table_name} CASCADE;"
    query.callback(sql, path)


@cli.command(help="Muestra el comando para conectarte manualmente a la base de datos")
@click.argument("path", required=True)
def connect(path):
    # docker-lens connect [path]	Muestra el comando de conexión

    if path == ".":
        path = os.getcwd()

    try:
        oc_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(oc_path)

    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return
    
    db_type = c.get("db_type", "postgres")
    host = c.get("host", "localhost")
    port = c.get("port", "")
    user = c.get("user", "")
    password = c.get("password", "")
    database = c.get("database", "")

    if db_type in ("mysql", "mariadb"):
        cmd = f"mysql -h{host} -P{port} -u{user}"
        if password:
            cmd += f" -p{password}"
        if database:
            cmd += f" -D{database}"
        click.echo(cmd)
    elif db_type == "mongo":
        if user:
            click.echo(f"mongosh 'mongodb://{user}:{password}@{host}:{port}/{database}'")
        else:
            click.echo(f"mongosh 'mongodb://{host}:{port}/{database}'")
    elif db_type == "sqlite":
        click.echo(f"sqlite3 {database}")
    else:
        # PostgreSQL (default)
        click.echo(f"psql -h{host} -p{port} -U{user} -d{database}")


@cli.command(help="Abre una sesión interactiva con la base de datos")
@click.argument("path", required=True)
def shell(path):
    # docker-lens shell [path]	Abre sesión interactiva según db_type

    if path == ".":
        path = os.getcwd()

    try:
        oc_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(oc_path)

    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return
    
    db_type = c.get("db_type", "postgres")
    host = c.get("host", "localhost")
    port = c.get("port", "")
    user = c.get("user", "")
    password = c.get("password", "")
    database = c.get("database", "")

    if db_type in ("mysql", "mariadb"):
        args = ["mysql", f"-h{host}", f"-P{port}", f"-u{user}"]
        if database:
            args.append(f"-D{database}")
        env = {**os.environ}
        if password:
            env["MYSQL_PWD"] = password
        subprocess.call(args, env=env)
    elif db_type == "mongo":
        if user:
            uri = f"mongodb://{user}:{password}@{host}:{port}/{database}"
        else:
            uri = f"mongodb://{host}:{port}/{database}"
        subprocess.call(["mongosh", uri])
    elif db_type == "sqlite":
        subprocess.call(["sqlite3", database])
    else:
        # PostgreSQL (default)
        subprocess.call(
            ["psql", f"-h{host}", f"-p{port}", f"-U{user}", f"-d{database}"],
            env={**os.environ, "PGPASSWORD": password}
        )


# Keep backward-compatible alias
@cli.command(help="Abre una sesión interactiva de psql (solo PostgreSQL)")
@click.argument("path", required=True)
def psql(path):
    """Backward-compatible alias for PostgreSQL interactive session."""
    if path == ".":
        path = os.getcwd()

    try:
        oc_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(oc_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    db_type = c.get("db_type", "postgres")
    if db_type != "postgres":
        click.echo(f"psql es solo para PostgreSQL. Esta base de datos es {db_type}.", err=True)
        click.echo(f"Usá 'docker-lens shell .' en su lugar.")
        return
    
    subprocess.call(
        ["psql", f"-hlocalhost", f"-p{c['port']}", f"-U{c['user']}", f"-d{c['database']}"],
        env={**os.environ, "PGPASSWORD": c['password']}
    )


@cli.command(help="Exporta una tabla a CSV")
@click.argument("table_name")
@click.option("-o", "--output", default=None, help="Archivo de salida (default: <tabla>.csv)")
@click.argument("path", required=True)
def export_csv(table_name, output, path):

    if path == ".":
        path = os.getcwd()

    if not output:
        output = f"{table_name}.csv"

    try:
        oc_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(oc_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    try:
        docker_lens.export_csv_py(c, table_name, output)
        click.echo(f"Exportado a {output}")
    except RuntimeError as e:
        click.echo(f"Error al exportar: {e}", err=True)


@cli.command(help="Exporta todas las tablas a CSV")
@click.argument("path", required=True)
def export_all(path):

    if path == ".":
        path = os.getcwd()
    try:
        oc_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(oc_path)

    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return
    
    tablas_raw = docker_lens.list_tables_py(c)
    db_type = c.get("db_type", "postgres")
    
    # Parse table names depending on db_type
    if db_type == "mongo":
        tablas = [line.strip() for line in tablas_raw.strip().split("\n")
                  if line.strip() and "No collections" not in line]
    elif db_type in ("mysql", "mariadb"):
        tablas = [line.split("|")[1].strip() if "|" in line else line.strip()
                  for line in tablas_raw.strip().split("\n")
                  if line.strip() and "Tables_in_" not in line and "+" not in line]
    elif db_type == "sqlite":
        tablas = [line.strip() for line in tablas_raw.strip().split("\n")
                  if line.strip() and "No tables" not in line]
    else:
        # PostgreSQL
        tablas = [line.split("|")[1].strip() for line in tablas_raw.strip().split("\n")
                  if "|" in line and "public" in line]
    
    for t in tablas:
        output = f"{t}.csv"
        docker_lens.export_csv_py(c, t, output)
        click.echo(f"✓ {output}")
        
    click.echo(f"\nExportadas {len(tablas)} tablas")


@cli.command(help="Exporta toda la base de datos a SQLite")
@click.option("-o", "--output", default=None, help="Archivo de salida (default: <db>.db)")
@click.argument("path", required=True)
def export_sqlite(output, path):

    if path == ".":
        path = os.getcwd()

    try:
        oc_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(oc_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    if not output:
        db_name = c.get("database", "export")
        output = f"{db_name}.db"

    db_type = c.get("db_type", "postgres")
    if db_type == "sqlite":
        click.echo("La base de datos ya es SQLite. Copiá el archivo directamente.", err=True)
        return

    try:
        docker_lens.export_to_sqlite_py(c, output)
        click.echo(f"Exportado a {output}")
    except RuntimeError as e:
        click.echo(f"Error al exportar: {e}", err=True)


if __name__ == '__main__':
    cli()

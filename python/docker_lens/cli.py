import subprocess

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
        credenciales = docker_lens.find_db_py(orchestator_path)
        click.echo(credenciales)
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
    

@cli.command(help="Ejecuta una consulta SQL custom")
@click.argument("query", required = True)
@click.argument("path", required = True)

def query(query, path):
    # docker-lens query "<sql>" [path]
    
    if path == ".":
        path = os.getcwd()

    try:
        orchestator_path = docker_lens.find_orchestrator_py(path)
    
    except RuntimeError:
        click.echo("No se encontró un archivo docker-compose.yml en este proyecto", err=True)
        return
    
    try:
        credenciales = docker_lens.find_db_py(orchestator_path)
        response = docker_lens.make_query_py(credenciales, query)
        click.echo(response)

    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)


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

    sql = f"SELECT * FROM {table_name} ORDER BY id DESC LIMIT {limit};"


    query.callback(sql, path)


@cli.command(help="Muestra las columnas, tipos y constraints de una tabla")
@click.argument("table_name")
@click.argument("path", required=True)
def schema(table_name, path):
    # docker-lens schema <tabla> [path]	Columnas, tipos, constraints
    
    if path == ".":
        path = os.getcwd()

    sql = f"SELECT column_name, data_type, is_nullable, column_default FROM information_schema.columns WHERE table_name = '{table_name}' ORDER BY ordinal_position;"


    query.callback(sql, path)


@cli.command(help="Cuenta cantidad de filas en una tabla")
@click.argument("table_name")
@click.argument("path", required=True)
def count(table_name, path):
    # docker-lens count <tabla> [path]	Cantidad de filas
    
    if path == ".":
        path = os.getcwd()

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

    sql = f"DROP TABLE {table_name} CASCADE;"

    query.callback(sql, path)

@cli.command(help="Muestra el comando psql para conectarte manualmente")
@click.argument("path", required=True)
def connect(path):
    # docker-lens psql [path]	Abre psql interactivo

    if path == ".":
        path = os.getcwd()

    try:
        oc_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(oc_path)

    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return
    
    click.echo(f"psql -hlocalhost -p{c['port']} -U{c['postgres_user']} -d{c['postgres_db']}")


@cli.command(help="Abre una sesión interactiva de psql")
@click.argument("path", required=True)
def psql(path):
    # docker-lens psql [path]	Abre psql interactivo

    if path == ".":
        path = os.getcwd()

    try:
        oc_path = docker_lens.find_orchestrator_py(path)
        c = docker_lens.find_db_py(oc_path)

    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return
    
    subprocess.call(
        ["psql", "-hlocalhost", f"-p{c['port']}", f"-U{c['postgres_user']}", f"-d{c['postgres_db']}"],
        env={**os.environ, "PGPASSWORD": c['postgres_password']}
    )


if __name__ == '__main__':
    cli()
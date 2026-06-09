import docker_lens
import click
import os
import subprocess

from .base import DbEngine
from .postgres_cli import PostgresEngine
from .mongo_cli import MongoEngine
from .mysql_cli import MysqlEngine
from .sqlite_cli import SqliteEngine

ENGINES = {
    "postgres": PostgresEngine,
    "mysql": MysqlEngine,
    "mariadb": MysqlEngine,
    "sqlite": SqliteEngine,
    "mongo": MongoEngine,
}


def get_engine(db_type: str) -> DbEngine:
    return ENGINES[db_type]()


@click.group()
def cli():
    """Docker-Lens: accedé a tu base de datos desde el terminal"""
    pass


@cli.command(help="Muestra las credenciales de la base de datos\n\nUso: docker-lens info PATH")
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


@cli.command(help="Cuenta todas las tablas de la base de datos\n\nUso: docker-lens tables PATH")
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


@cli.command(help="Ejecuta una consulta SQL custom\n\nUso: docker-lens query 'SELECT * FROM tabla' PATH")
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


@cli.command(help="Exporta una tabla a CSV\n\nUso: docker-lens export-csv TABLE_NAME PATH [-o OUTPUT]")
@click.argument("table_name")
@click.option("-o", "--output", default=None)
@click.argument("path")
def export_csv(table_name, output, path):
   
    if path == ".":
        path = os.getcwd()

    if not output:
        output = f"{table_name}.csv"

    try:
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return  

    try:
        docker_lens.export_csv_py(creds, table_name, output)
        click.echo(f"Exportado correctamente en {output}")
    except RuntimeError:
        click.echo("No fue posible realizar la exportacion", err=True)


@cli.command(help="Exporta todas las tablas a CSV\n\nUso: docker-lens export-all PATH [-o DIR]")
    
    if path == ".":
        path = os.getcwd()

    try:
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return  

    raw_tables = docker_lens.list_tables_py(creds)

    engine = get_engine(creds["db_type"])
    parsed_tables = engine.parse_tables(raw_tables)

    os.makedirs(output, exist_ok=True)

    try:
        for table in parsed_tables:
            file_path = os.path.join(output, f"{table}.csv")
            docker_lens.export_csv_py(creds, table, file_path)

        click.echo(f"Exportadas {len(parsed_tables)} tablas en {output}")
    except RuntimeError:
        click.echo("No fue posible realizar la exportacion", err=True)
    

@cli.command(help="Exporta la base de datos a SQLite\n\nUso: docker-lens export-sqlite PATH [-o OUTPUT]")
@click.option("-o", "--output", default=None)
@click.argument("path")
def export_sqlite(output, path):

    if path == ".":
        path = os.getcwd()

    if not output:
        output = "database.db"

    try:
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return  

    try:
        docker_lens.export_to_sqlite_py(creds, output)
        click.echo(f"Exportado correctamente en {output}")

    except RuntimeError:
        click.echo("No fue posible realizar la exportacion", err=True)


# Funciones de la interfaz:

@cli.command(help="Muestra el comando para conectarte a la base de datos\n\nUso: docker-lens connect PATH")
@click.argument("path")
def connect(path):

    if path == ".":
        path = os.getcwd()

    try:
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return  

    engine = get_engine(creds["db_type"])
    
    result = engine.connect_command(creds["port"], creds["user"], creds["database"])

    click.echo(result)

@cli.command(help="Abre una sesión interactiva con la DB\n\nUso: docker-lens shell PATH")
@click.argument("path")
def shell(path):

    if path == ".":
        path = os.getcwd()

    try:
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return  

    engine = get_engine(creds["db_type"])
    args = engine.shell_args(creds["host"], creds["port"], creds["user"], creds["password"], creds["database"])
    env = engine.shell_env(creds["password"])

    subprocess.call(args, env=env)

@cli.command(help="Muestra las primeras N filas de una tabla\n\nUso: docker-lens head TABLE_NAME PATH [-n LIMIT]")
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
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return
    
    engine = get_engine(creds["db_type"])
    query = engine.head(table_name, limit)
    result = docker_lens.make_query_py(creds, query)

    click.echo(result)


@cli.command(help="Muestra las últimas N filas de una tabla\n\nUso: docker-lens tail TABLE_NAME PATH [-n LIMIT]")
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
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    engine = get_engine(creds["db_type"])
    query = engine.tail(table_name, limit)
    result = docker_lens.make_query_py(creds, query)

    click.echo(result)


@cli.command(help="Muestra las columnas, tipos y constraints de una tabla\n\nUso: docker-lens schema TABLE_NAME PATH")
@click.argument("table_name")
@click.argument("path", required=True)
def schema(table_name, path):
    # docker-lens schema <tabla> [path]	Columnas, tipos, constraints
    
    if path == ".":
        path = os.getcwd()

    try:
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    engine = get_engine(creds["db_type"])
    query = engine.schema(table_name, creds["database"])
    result = docker_lens.make_query_py(creds, query)

    click.echo(result)

@cli.command(help="Cuenta cantidad de filas en una tabla\n\nUso: docker-lens count TABLE_NAME PATH")
@click.argument("table_name")
@click.argument("path", required=True)
def count(table_name, path):
    # docker-lens count <tabla> [path]	Cantidad de filas
    
    if path == ".":
        path = os.getcwd()

    try:
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    engine = get_engine(creds["db_type"])
    query = engine.count(table_name)
    result = docker_lens.make_query_py(creds, query)

    click.echo(result)

@cli.command(help="Vacía una tabla (CASCADE si tiene dependencias)\n\nUso: docker-lens truncate TABLE_NAME PATH [--force]")
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
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    engine = get_engine(creds["db_type"])
    query = engine.truncate(table_name)
    result = docker_lens.make_query_py(creds, query)

    click.echo(result)

@cli.command(help="Elimina una tabla (CASCADE si tiene dependencias)\n\nUso: docker-lens drop TABLE_NAME PATH [--force]")
@click.argument("table_name")
@click.option("--force", is_flag=True, help="Saltar confirmacion")
@click.argument("path", required=True)
def drop(table_name, force, path):
    # docker-lens drop <tabla> [path]	Elimina la tabla
    
    if not force:
        click.confirm(f"¿Eliminar la tabla '{table_name}'? Esto es irreversible.", abort=True)

    if path == ".":
        path = os.getcwd()

    try:
        base_path = docker_lens.find_orchestrator_py(path)
        creds = docker_lens.find_db_py(base_path)
    except RuntimeError:
        click.echo("Base de datos inaccesible", err=True)
        return

    engine = get_engine(creds["db_type"])
    query = engine.drop(table_name)
    result = docker_lens.make_query_py(creds, query)

    click.echo(result)
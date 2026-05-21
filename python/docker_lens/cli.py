import docker_lens
import click
import os

@click.group()
def cli():
    """Docker-Lens: La puritita calidad"""
    pass

#"/home/frandev/Documentos/Proyecto-Asignatura-Web"

@cli.command()
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
  


@cli.command()
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
    

@cli.command()
def query():
    # docker-lens query "" [path]
    print("Aqui te va la response")

if __name__ == '__main__':
    cli()
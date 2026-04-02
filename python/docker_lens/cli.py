"""Docker-Lens CLI - Terminal interface for Docker-Lens."""
import argparse
import json
import subprocess
import sys

import docker_lens
from docker_lens import __version__


def _resolve_credentials(path: str) -> tuple[str, str, str, str]:
    """
    Resolve database credentials from a project path.
    
    Calls find_orchestrator to locate docker-compose.yml, then find_db to get credentials.
    
    Returns:
        tuple: (user, password, db, port)
    
    Raises:
        SystemExit(1): If docker-compose not found or other errors.
    """
    try:
        orchestrator = docker_lens.find_orchestrator(path)
        if not orchestrator:
            print("Error: docker-compose.yml not found in the specified path.", file=sys.stderr)
            sys.exit(1)
        
        db = docker_lens.find_db(orchestrator)
        return (
            db['POSTGRES_USER'],
            db['POSTGRES_PASSWORD'],
            db['POSTGRES_DB'],
            db['port']
        )
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except RuntimeError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


def cmd_find_db(args: argparse.Namespace) -> int:
    """Handler for 'find-db' subcommand."""
    try:
        orchestrator = docker_lens.find_orchestrator(args.path)
        if not orchestrator:
            print("Error: docker-compose.yml not found in the specified path.", file=sys.stderr)
            return 1
        
        db = docker_lens.find_db(orchestrator)
        
        if args.json:
            print(json.dumps(db, indent=2))
        else:
            print(f"Port:     {db['port']}")
            print(f"User:     {db['POSTGRES_USER']}")
            print(f"Password: {db['POSTGRES_PASSWORD']}")
            print(f"DB Name:  {db['POSTGRES_DB']}")
        
        return 0
    except FileNotFoundError as e:
        print(f"Error: docker-compose not found - {e}", file=sys.stderr)
        return 1
    except RuntimeError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


def cmd_tables(args: argparse.Namespace) -> int:
    """Handler for 'tables' subcommand."""
    try:
        user, password, db, port = _resolve_credentials(args.path)
        result = docker_lens.get_tables_return(user, password, db, port)
        print(result, end='')
        return 0
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1
    except RuntimeError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError:
        print("Error: psql not found. Install PostgreSQL client tools.", file=sys.stderr)
        return 1
    except Exception as e:
        error_msg = str(e).lower()
        if 'psql' in error_msg or 'subprocess' in error_msg:
            print("Error: psql is required but not found. Please install PostgreSQL client.", file=sys.stderr)
        else:
            print(f"Error: {e}", file=sys.stderr)
        return 1


def cmd_query(args: argparse.Namespace) -> int:
    """Handler for 'query' subcommand."""
    try:
        user, password, db, port = _resolve_credentials(args.path)
        result = docker_lens.execute_query(user, password, db, port, args.query)
        print(result, end='')
        return 0
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1
    except RuntimeError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError:
        print("Error: psql not found. Install PostgreSQL client tools.", file=sys.stderr)
        return 1
    except Exception as e:
        error_msg = str(e).lower()
        if 'psql' in error_msg or 'subprocess' in error_msg:
            print("Error: psql is required but not found. Please install PostgreSQL client.", file=sys.stderr)
        else:
            print(f"Error: {e}", file=sys.stderr)
        return 1


def main() -> None:
    """Entry point for the docker-lens CLI."""
    parser = argparse.ArgumentParser(
        prog='docker-lens',
        description='Docker-Lens: Find and query databases in Docker Compose projects.'
    )
    parser.add_argument(
        '--version',
        action='version',
        version=f'docker-lens {__version__}'
    )
    
    subparsers = parser.add_subparsers(dest='command', help='Available commands')
    
    # find-db subcommand
    parser_find_db = subparsers.add_parser(
        'find-db',
        help='Discover database configuration in docker-compose.yml'
    )
    parser_find_db.add_argument(
        'path',
        nargs='?',
        default='.',
        help='Path to project directory (default: current directory)'
    )
    parser_find_db.add_argument(
        '--json',
        action='store_true',
        help='Output credentials as JSON'
    )
    parser_find_db.set_defaults(func=cmd_find_db)
    
    # tables subcommand
    parser_tables = subparsers.add_parser(
        'tables',
        help='List all tables in the database'
    )
    parser_tables.add_argument(
        'path',
        nargs='?',
        default='.',
        help='Path to project directory (default: current directory)'
    )
    parser_tables.set_defaults(func=cmd_tables)
    
    # query subcommand
    parser_query = subparsers.add_parser(
        'query',
        help='Execute a SQL query on the database'
    )
    parser_query.add_argument(
        'path',
        nargs='?',
        default='.',
        help='Path to project directory (default: current directory)'
    )
    parser_query.add_argument(
        'query',
        help='SQL query to execute'
    )
    parser_query.set_defaults(func=cmd_query)
    
    args = parser.parse_args()
    
    if args.command is None:
        parser.print_help()
        sys.exit(1)
    
    sys.exit(args.func(args))


if __name__ == '__main__':
    main()

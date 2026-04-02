# Importar el módulo Rust
import docker_lens as _rust_module

# Re-export las funciones
from docker_lens import (
    find_orchestrator,
    find_db,
    get_tables,
    get_tables_return,
    execute_query
)

__version__ = "0.1.0"

__all__ = [
    "find_orchestrator",
    "find_db", 
    "get_tables",
    "get_tables_return",
    "execute_query"
]
import re

def sanitize_table_name(name):
    if not re.match(r'^[a-zA-Z_][a-zA-Z0-9_]*$', name):
        raise ValueError(f"Nombre de tabla inválido: {name}")
    return name
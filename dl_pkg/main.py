import docker_lens

# 1. Encontrar orquestador
orch = docker_lens.find_orchestrator("/home/frandev/Documentos/Proyecto-Asignatura-Web")
print(f"Orquestador: {orch}")
# 2. Encontrar DB (devuelve dict)
db = docker_lens.find_db("/home/frandev/Documentos/Proyecto-Asignatura-Web")
print(f"User: {db['POSTGRES_USER']}")
print(f"Password: {db['POSTGRES_PASSWORD']}")
print(f"DB: {db['POSTGRES_DB']}")
print(f"Port: {db['port']}")
# 3. Listar tablas
tables = docker_lens.get_tables_return(
    db['POSTGRES_USER'], 
    db['POSTGRES_PASSWORD'], 
    db['POSTGRES_DB'], 
    db['port']
)
print(tables)
# 4. Query personalizada
result = docker_lens.execute_query(
    db['POSTGRES_USER'],
    db['POSTGRES_PASSWORD'],
    db['POSTGRES_DB'],
    db['port'],
    "SELECT * FROM quality_data_color LIMIT 5;"
)
print(result)
import docker_lens

credenciales = docker_lens.find_db_py("/home/frandev/Documentos/Proyecto-Asignatura-Web")

print (credenciales)


tablas = docker_lens.list_tables_py(credenciales)

print(tablas)
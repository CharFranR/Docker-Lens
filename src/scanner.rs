use std::io::{Error, ErrorKind};
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

// Este archivo contiene todas las funciones respecto al escaneo de las carpetas del proyecto donde se invoque la libreria

pub fn file_is_here(file_path: &Path, target: &str) -> bool {
    // Determina si cierto archivo se encuentra (o no) en la ruta proporcionada
    file_path.join(target).exists()
}

pub fn find_ochestor_folder(file_path: &Path) -> std::io::Result<PathBuf> {
    // Busca la carpeta raiz del proyecto, en este caso la que contenga el .git (Esta implementacion todavia es debatible)

    let target = ".git";
    let mut contador = 0;
    let mut current_dir = file_path.to_path_buf();

    loop {
        if file_is_here(&current_dir, target) {
            return Ok(current_dir);
        }

        contador += 1;

        // 3. Si después de 5 carpetas no se encuentra el .git, el orquestador es inaccesible.
        if contador >= 5 {
            break;
        }

        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => break,
        }
    }

    Err(Error::new(
        ErrorKind::NotFound,
        "No se encontró la carpeta raiz",
    ))
}

pub fn path_recursion_search(init_path: &Path, target: &str) -> std::io::Result<PathBuf> {
    for entry in WalkDir::new(init_path)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == target {
            return Ok(entry.path().to_path_buf());
        }
    }
    // 5. Si no encuentra el orquestador, el orquestador es inaccesible.
    Err(Error::new(
        ErrorKind::NotFound,
        "No fue posible encontrar el orquestador",
    ))
}

pub fn find_container_orchestrator(file_path: &Path) -> std::io::Result<PathBuf> {
    // Utiliza las funciones anteriores para realizar la búsqueda del archivo docker-compose.yml

    let target = "docker-compose.yml";

    // 1. Comprobar en la dirección de orígen.
    if file_is_here(file_path, target) {
        return Ok(file_path.to_path_buf());
    }

    // 2. Si no se encuentra, realizar Upward Discovery hasta encontrar el .git.

    let init_folder = find_ochestor_folder(file_path)?;

    // 3. Si encuentra el .git comprobar en la dirección actual de búsqueda.

    if file_is_here(&init_folder, target) {
        return Ok(init_folder);
    }

    // 4. Si no encuentra el orquestador, realizar búsqueda mediante un método recursivo.

    let docker_compose_path = path_recursion_search(&init_folder, target);

    return docker_compose_path;
}

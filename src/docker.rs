use bollard::Docker;
use bollard::query_parameters::ListImagesOptionsBuilder;


pub async fn docker_version (){
    let docker = Docker::connect_with_local_defaults().unwrap();

    match docker.version().await{
        Ok (v) => println!("La version de docker es: {:?}", v),
        Err (e) => println!("Error a obtener la version de docker {}", e),
    }
}

pub async fn docker_images (){
    let docker = Docker::connect_with_local_defaults().unwrap();

    let options = ListImagesOptionsBuilder::default()
        .all(true)
        .build();

    let images = &docker.list_images(Some(options)).await.unwrap();

    for image in images {
        println!("-> {:?}", image);
    }
}
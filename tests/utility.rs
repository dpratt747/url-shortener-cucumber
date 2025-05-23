#![allow(warnings)]
use crate::URLShortenerWorld;
use bollard::Docker;
use bollard::container::StartContainerOptions;
use bollard::models::ImageSummary;
use bollard::query_parameters::{ListContainersOptions, ListImagesOptions, StopContainerOptions};
use rand::Rng;
use rand::distr::Alphanumeric;
use std::collections::HashMap;
use std::net::TcpListener;

pub fn generate_random_url(base: &str) -> String {
    let random_part: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    format!("{}/{}", base, random_part)
}

fn generate_random_word(length: usize) -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::rng();

    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

async fn get_container_id(
    docker: &Docker,
    filters: HashMap<String, Vec<String>>,
) -> Option<String> {
    let containers = docker
        .list_containers(Some(ListContainersOptions {
            all: true, // Include stopped containers
            filters: Some(filters.clone()),
            ..Default::default()
        }))
        .await
        .unwrap();

    if let Some(container) = containers.first() {
        if let Some(id) = container.id.as_ref() {
            return Some(id.clone());
        }
    }
    None
}

async fn wait_for_container_to_start_running(docker: &Docker, container_id: &str) {
    loop {
        let inspect = docker
            .inspect_container(
                container_id,
                None::<bollard::container::InspectContainerOptions>,
            )
            .await
            .expect("Unable to inspect container");

        if let Some(state) = inspect.state {
            if state.running.unwrap_or(false) {
                return ();
            }
        }
    }
}

fn get_available_host_port() -> Option<u16> {
    // Binding to port 0 lets the OS assign an available port
    if let Ok(listener) = TcpListener::bind("127.0.0.1:0") {
        if let Ok(addr) = listener.local_addr() {
            return Some(addr.port());
        }
    }
    None
}

/// This function takes a URLShortenerWorld and assigns a random container_name and available container_port.
/// This world is created per Scenario
pub async fn create_and_start_url_shortener_docker_container(world: &mut URLShortenerWorld, image_name: &str) {
    // let image_name = "url_shortener_rust"; // todo: expected image name
    world.container_name = generate_random_word(10).to_string();
    world.container_port = get_available_host_port().expect("Unable to get available host port");

    let docker = Docker::connect_with_socket_defaults().expect("Unable to connect to docker");

    let docker_images: Vec<ImageSummary> = docker
        .list_images(None::<ListImagesOptions>)
        .await
        .expect("Unable to list images");

    let image_exists = docker_images
        .iter()
        .any(|img| img.repo_tags.iter().any(|s| s.contains(image_name)));

    if !image_exists {
        panic!(
            "There is no docker image found! Expected the following image name {}",
            image_name
        );
    }

    let mut filters_map: HashMap<String, Vec<String>> = HashMap::new();
    filters_map.insert("name".to_string(), vec![world.container_name.clone()]);

    // Configure the container
    let config = bollard::models::ContainerCreateBody {
        image: Some(format!("{}:latest", image_name)), // todo: can configure the image version here
        host_config: Some(bollard::models::HostConfig {
            port_bindings: Some({
                let mut port_bindings = HashMap::new();
                port_bindings.insert(
                    "8080/tcp".to_string(), // todo: internal docker port
                    Some(vec![bollard::models::PortBinding {
                        host_ip: Some("0.0.0.0".to_string()), // Bind to all interfaces
                        host_port: Some(world.container_port.to_string()), // todo: host machine port
                    }]),
                );
                port_bindings
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let create_options = bollard::query_parameters::CreateContainerOptionsBuilder::default()
        .name(&world.container_name)
        .build();

    docker
        .create_container(Some(create_options), config)
        .await
        .expect("Could not create container");

    docker
        .start_container(&world.container_name, None::<StartContainerOptions<String>>)
        .await
        .expect("Could not start container");

    // need to wait for the container to be ready
    wait_for_container_to_start_running(
        &docker,
        &get_container_id(&docker, filters_map.clone())
            .await
            .expect("Unable to get container id"),
    )
    .await;
}

pub async fn stop_docker_container(container_name: &str) {
    let docker = Docker::connect_with_socket_defaults().expect("Unable to connect to docker");
    docker
        .stop_container(container_name, None::<StopContainerOptions>)
        .await
        .expect("Unable to stop the container");
}

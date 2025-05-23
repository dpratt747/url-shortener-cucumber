use crate::models::URLShortenerWorld;
use bollard::models::ImageSummary;
use bollard::query_parameters::{ListImagesOptions, LogsOptionsBuilder, RemoveContainerOptions};
use bollard::Docker;
use futures::StreamExt;
use rand::distr::Alphanumeric;
use rand::Rng;
use std::collections::HashMap;
use std::net::TcpListener;
use std::time::Duration;
use tokio::time::timeout;

pub fn generate_random_url(base: &str) -> String {
    let rng = rand::rng();
    let random_part: String = rng
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

async fn wait_for_log_message(
    docker: &Docker,
    container_name: &str,
    target_message: &str,
    timeout_duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = Some(
        LogsOptionsBuilder::new()
            .stdout(true)
            .stderr(true)
            .follow(true)
            .build(),
    );

    let mut stream = docker.logs(container_name, options);

    timeout(timeout_duration, async {
        while let Some(Ok(log)) = stream.next().await {
            let log_message = log.to_string();
            if log_message.contains(target_message) {
                return Ok(());
            }
        }
        Err("Stream ended without finding the target message".into())
    })
    .await
    .unwrap_or_else(|_| Err("Timeout reached while waiting for log message".into()))
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
pub async fn create_and_start_url_shortener_docker_container(
    world: &mut URLShortenerWorld,
    image_name: &str,
    container_internal_port: &str,
) {
    world.container_name = generate_random_word(10).to_string();
    world.container_host_port = get_available_host_port().expect("Unable to get available host port");

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
                    format!("{}/tcp", container_internal_port), //todo: internal docker port
                    Some(vec![bollard::models::PortBinding {
                        host_ip: Some("0.0.0.0".to_string()), // Bind to all interfaces
                        host_port: Some(world.container_host_port.to_string()), // todo: host machine port
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
        .start_container(&world.container_name, None::<bollard::query_parameters::StartContainerOptions>)
        .await
        .expect("Could not start container");

    // can sleep here, wait for the appearance of a log message or configure health checks
    // std::thread::sleep(std::time::Duration::from_millis(500));
    wait_for_log_message(
        &docker,
        world.container_name.as_str(),
        "The server has been started",
        Duration::from_secs(4),
    )
    .await
    .expect("Error whilst waiting for the container log messages");
}

pub async fn stop_docker_container(container_name: &str) {
    let docker = Docker::connect_with_socket_defaults().expect("Unable to connect to docker");

    let options = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };

    docker
        .remove_container(container_name, Some(options))
        .await
        .expect("Unable to remove container");
}

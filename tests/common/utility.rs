use bollard::Docker;
use bollard::models::ImageSummary;
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, CreateImageOptionsBuilder, ListImagesOptions,
    LogsOptionsBuilder, RemoveContainerOptions, StartContainerOptions,
};
use futures::StreamExt;
use rand::Rng;
use rand::distr::Alphanumeric;
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
    (1..=length)
        .map(|_| {
            let idx = rand::rng().random_range(0..CHARSET.len());
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
pub async fn create_and_start_docker_container(
    image_name: &str,
    container_internal_port: &str,
    expected_start_message: &str,
    environment_variables: Option<Vec<String>>,
) -> (String, u16) {
    let container_name = generate_random_word(10).to_string();
    let container_host_port = get_available_host_port().expect("Unable to get available host port");

    let docker = Docker::connect_with_socket_defaults().expect("Unable to connect to docker");

    let docker_images: Vec<ImageSummary> = docker
        .list_images(None::<ListImagesOptions>)
        .await
        .expect("Unable to list images");

    let image_exists = docker_images
        .iter()
        .any(|img| img.repo_tags.iter().any(|s| s.contains(image_name)));

    if !image_exists {
        log::info!("Image is not found attempting to pull the image");

        let create_image_options = Some(
            CreateImageOptionsBuilder::default()
                .from_image(&image_name)
                .tag("latest")
                .build(),
        );

        let mut stream = docker.create_image(create_image_options, None, None);

        while let Some(message) = stream.next().await {
            match message {
                Ok(output) => {
                    if let Some(status) = output.status {
                        log::info!("Status: {}", status);
                    }
                    if let Some(progress) = output.progress {
                        log::info!("Progress: {}", progress);
                    }
                }
                Err(e) => panic!(
                    "There is no docker image found! Expected the following image name [{}] {}",
                    image_name, e
                ),
            }
        }
    }

    let mut filters_map: HashMap<String, Vec<String>> = HashMap::new();
    filters_map.insert("name".to_string(), vec![container_name.clone()]);

    // Configure the container
    let config = bollard::models::ContainerCreateBody {
        image: Some(format!("{}:latest", image_name)), // todo: can configure the image version here
        env: environment_variables,
        host_config: Some(bollard::models::HostConfig {
            port_bindings: Some({
                let mut port_bindings = HashMap::new();
                port_bindings.insert(
                    format!("{}/tcp", container_internal_port), //todo: internal docker port
                    Some(vec![bollard::models::PortBinding {
                        host_ip: Some("0.0.0.0".to_string()), // Bind to all interfaces
                        host_port: Some(container_host_port.to_string()), // todo: host machine port
                    }]),
                );
                port_bindings
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let create_options = Some(
        CreateContainerOptionsBuilder::default()
            .name(&container_name)
            .build(),
    );

    docker
        .create_container(create_options, config)
        .await
        .expect("Could not create container");

    docker
        .start_container(&container_name, None::<StartContainerOptions>)
        .await
        .expect("Could not start container");

    // can sleep here, wait for the appearance of a log message or configure health checks
    // std::thread::sleep(std::time::Duration::from_millis(500));
    wait_for_log_message(
        &docker,
        container_name.as_str(),
        expected_start_message,
        Duration::from_secs(4),
    )
    .await
    .expect("Error whilst waiting for the container log messages");

    (container_name, container_host_port)
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
        .expect("Unable to remove container - are you sure it is running?");
}

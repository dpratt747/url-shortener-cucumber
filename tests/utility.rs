#![allow(warnings)]
use bollard::query_parameters::ListContainersOptions;
use bollard::Docker;
use rand::distr::Alphanumeric;
use rand::Rng;
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

pub fn generate_random_word(length: usize) -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::rng();

    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub async fn get_container_id(
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

pub async fn wait_for_container_to_start_running(docker: &Docker, container_id: &str) {
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

pub fn get_available_host_port() -> Option<u16> {
    // Binding to port 0 lets the OS assign an available port
    if let Ok(listener) = TcpListener::bind("127.0.0.1:0") {
        if let Ok(addr) = listener.local_addr() {
            return Some(addr.port());
        }
    }
    None
}

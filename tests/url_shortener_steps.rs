#![allow(warnings)]
use bollard::Docker;
use bollard::container::{RestartContainerOptions, StartContainerOptions};
use bollard::models::ImageSummary;
use bollard::query_parameters::{
    ListContainersOptions, ListImagesOptions, RestartContainerOptionsBuilder, StopContainerOptions,
};
use cucumber::{Cucumber, World as _, given, then, when};
use rand::Rng;
use rand::distr::Alphanumeric;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::net::TcpListener;

mod utility;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GetAllShortenUrlResponse(HashMap<String, String>);

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct ShortenUrlRequest {
    longUrl: String,
}

#[derive(cucumber::World, Debug, Default)]
pub struct URLShortenerWorld {
    long_url: String,
    shorten_url_status_code: u16,
    get_shortened_url_response: GetAllShortenUrlResponse,
    container_port: u16,
    container_name: String,
}

#[given(expr = "I have a running url shortener docker container")]
async fn create_shortener_docker_container(world: &mut URLShortenerWorld) {
    let image_name = "url_shortener_rust"; // todo: expected image name
    world.container_name = utility::generate_random_word(10).to_string();
    world.container_port =
        utility::get_available_host_port().expect("Unable to get available host port");

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
    utility::wait_for_container_to_start_running(
        &docker,
        &utility::get_container_id(&docker, filters_map.clone())
            .await
            .expect("Unable to get container id"),
    )
    .await;
}

#[given(expr = "I have a long URL {string}")]
async fn have_a_long_url(world: &mut URLShortenerWorld, url: String) {
    world.long_url = url;
}

#[when(expr = "I make a request to the shorten URL endpoint")]
async fn send_shorten_request(world: &mut URLShortenerWorld) {
    let client: Client = Client::new();
    let endpoint = format!("http://localhost:{}/v1/shorten", world.container_port.to_string());
    let body = ShortenUrlRequest {
        longUrl: world.long_url.clone(),
    };
    let json_body: serde_json::Value = serde_json::to_value(&body).expect("Unable to serialize json");

    let response = client
        .post(&endpoint)
        .json(&json_body)
        .send()
        .await
        .expect("Could not send request");

    match response.status() {
        status if status.is_success() => {
            log::info!("Request successful! Status: {}", status);
            if let Ok(text) = response.text().await {
                log::info!("Response body: {}", text);
            }
            world.shorten_url_status_code = status.as_u16();
        }
        status => {
            log::error!("Request failed! Status: {}", status);
            if let Ok(text) = response.text().await {
                log::error!("Error response: {}", text);
            }
            world.shorten_url_status_code = status.as_u16();
        }
    }
}

#[then(expr = "I get a {int} status code")]
async fn post_shorten_url_result(
    world: &mut URLShortenerWorld,
    expected: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(world.shorten_url_status_code, expected);
    Ok(())
}

#[given(expr = "I make {int} requests to the shorten URL endpoint")]
async fn post_shorten_n_times(world: &mut URLShortenerWorld, number_of_requests: u16) {
    let client: Client = Client::new();
    let endpoint = format!("http://localhost:{}/v1/shorten", world.container_port.to_string());

    for _ in 0..number_of_requests {
        let body = ShortenUrlRequest {
            longUrl: utility::generate_random_url("http://some_domain"),
        };
        let json_body: serde_json::Value = serde_json::to_value(&body).expect("Unable to serialize json");

        let response = client
            .post(&endpoint)
            .json(&json_body)
            .send()
            .await
            .expect("Could not send request");

        match response.status() {
            status if status.is_success() => {
                log::info!("Request successful! Status: {}", status);
                if let Ok(text) = response.text().await {
                    log::info!("Response body: {}", text);
                }
            }
            status => {
                log::error!("Request failed! Status: {}", status);
                if let Ok(text) = response.text().await {
                    log::error!("Error response: {}", text);
                }
            }
        }
    }
}

#[when(expr = "I make a request to get all shortened endpoints")]
async fn get_all_shortened_urls(world: &mut URLShortenerWorld) {
    let client: Client = Client::new();
    let endpoint = format!("http://localhost:{}/v1/all", world.container_port.to_string());

    let response = client.get(&endpoint).send().await.expect("Could not send request");
    let status = response.status();
    let response_text = response.text().await.expect("Could not get response text");

    world.get_shortened_url_response = serde_json::from_str(&response_text).expect("Could not deserialize json");

    match status {
        status if status.is_success() => {
            log::info!("Request successful! Status: {}", status);
            log::info!("Response body: {}", response_text);
        }
        status => {
            log::error!("Request failed! Status: {}", status);
            log::error!("Error response: {}", response_text);
        }
    }
}

#[then(expr = "I get {int} shorten url responses")]
async fn get_all_shortened_urls_equals_n_responses(
    world: &mut URLShortenerWorld,
    expected_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        world.get_shortened_url_response.0.len(),
        expected_count
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();
    log::info!("Running feature files");
    URLShortenerWorld::cucumber()
        .after(|_feature, _rule, _scenario, _ev, _world| {
            Box::pin(async move {
                let docker =
                    Docker::connect_with_socket_defaults().expect("Unable to connect to docker");
                if let Some(world) = _world {
                    docker
                        .stop_container(&world.container_name, None::<StopContainerOptions>)
                        .await
                        .expect("Unable to stop the container");
                }
            })
        })
        .run_and_exit("tests/features/url_shortener/url_shortener.feature")
        .await;

    // URLShortenerWorld::run("tests/features/url_shortener/url_shortener.feature").await;
    log::info!("Finished running feature files");
    log::logger().flush();
}

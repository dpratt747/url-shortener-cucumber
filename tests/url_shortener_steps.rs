use bollard::models::ImageSummary;
use cucumber::{given, then, when, World as _};
use rand::distr::Alphanumeric;
use rand::Rng;
use reqwest::Client;
use serde::de::Unexpected::Option;
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use bollard::container::RestartContainerOptions;

const API: &str = "http://localhost:8080";

fn generate_random_url(base: &str) -> String {
    let random_part: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    format!("{}/{}", base, random_part)
}

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
}

#[given(expr = "I have a clean url shortener instance")]
async fn clean_shortener_service(_: &mut URLShortenerWorld) {
    let image_name = "url_shortener_rust";
    let container_name = "url_shortener_rust_container";
    let docker = bollard::Docker::connect_with_socket_defaults().unwrap();

    let docker_images: Vec<ImageSummary> = docker
        .list_images(None::<bollard::query_parameters::ListImagesOptions>)
        .await
        .unwrap();

    let image_exists = docker_images
        .iter()
        .any(|img| img.repo_tags.iter().any(|s| s.contains(image_name)));

    println!("image exists: {}", image_exists);
    println!("image exists: {}", image_exists);
    println!("image exists: {}", image_exists);

    let mut filters_map = HashMap::new();
    filters_map.insert("name".to_string(), vec![container_name.to_string()]);

    let containers = docker
        .list_containers(Some(bollard::query_parameters::ListContainersOptions {
            all: true, // Include stopped containers
            filters: Some(filters_map),
            ..Default::default()
        }))
        .await
        .unwrap();

    // Configure the container
    let config = bollard::models::ContainerCreateBody {
        image: Some(format!("{}:latest", image_name)),
        host_config: Some(bollard::models::HostConfig {
            port_bindings: Some({
                let mut port_bindings = HashMap::new();
                port_bindings.insert(
                    "8080/tcp".to_string(),
                    Some(vec![bollard::models::PortBinding {
                        host_ip: Some("0.0.0.0".to_string()), // Bind to all interfaces
                        host_port: Some("8080".to_string()),  // Map to host port 8080
                    }]),
                );
                port_bindings
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let create_options = bollard::query_parameters::CreateContainerOptionsBuilder::default()
        .name(container_name)
        .build();

    let container = docker
        .create_container(Some(create_options), config)
        .await
        .expect("Could not create container");

    if containers.is_empty() && image_exists {
        println!("no containers found");
        println!("no containers found");
        println!("no containers found");

        // run the image
        docker
            .start_container(
                &container.id,
                None::<bollard::query_parameters::StartContainerOptions>,
            )
            .await
            .unwrap();
        // docker.start_container(&container.id, None).await.expect("TODO: panic message");
        println!("containers started");
        println!("containers started");
        println!("containers started");
    } else if !containers.is_empty() {
        // restart the container
        docker
            .restart_container(
                &container.id,
                None::<bollard::query_parameters::RestartContainerOptions>,
                // RestartContainerOptions
                // None,
            )
            .await
            .unwrap();
        println!("containers restarted");
        println!("containers restarted");
        println!("containers restarted");
    } else {
        // create an image?
        panic!("image not found");
    }
}

#[given(expr = "I have a long URL {string}")]
async fn have_a_long_url(world: &mut URLShortenerWorld, url: String) {
    world.long_url = url;
}

#[when(expr = "I make a request to the shorten URL")]
async fn send_shorten_request(world: &mut URLShortenerWorld) {
    let client: Client = Client::new();
    let endpoint: String = format!("{}/v1/shorten", API);
    let body = ShortenUrlRequest {
        longUrl: world.long_url.clone(),
    };
    let json_body: serde_json::Value = serde_json::to_value(&body).unwrap();

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
async fn post_shorten_n_times(_: &mut URLShortenerWorld, number_of_requests: u16) {
    let client: Client = Client::new();
    let endpoint: String = format!("{}/v1/shorten", API);

    for i in 0..number_of_requests {
        let body = ShortenUrlRequest {
            longUrl: generate_random_url("http://some_domain"),
        };
        let json_body: serde_json::Value = serde_json::to_value(&body).unwrap();

        let response = client
            .post(&endpoint)
            .json(&json_body)
            .send()
            .await
            .unwrap();

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
    let endpoint: String = format!("{}/v1/all", API);

    let response = client.get(&endpoint).send().await.unwrap();
    let status = response.status();
    let response_text = response.text().await.unwrap();

    world.get_shortened_url_response = serde_json::from_str(&response_text).unwrap();

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
    expected_count: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        world.get_shortened_url_response.0.len(),
        expected_count as usize
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();
    log::info!("Running feature files");
    URLShortenerWorld::run("tests/features/url_shortener/url_shortener.feature").await;
    log::info!("Finished running feature files");
    log::logger().flush();
}

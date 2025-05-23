#![allow(warnings)]
use bollard::container::{RestartContainerOptions, StartContainerOptions};
use bollard::models::ImageSummary;
use bollard::query_parameters::{
    ListContainersOptions, ListImagesOptions, RestartContainerOptionsBuilder, StopContainerOptions,
};
use bollard::Docker;
use cucumber::{given, then, when, Cucumber, World as _};
use rand::distr::Alphanumeric;
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::net::TcpListener;
mod common;

use common::models::{ShortenUrlRequest, URLShortenerWorld};
use common::*;

#[given(expr = "I have a long URL {string}")]
async fn have_a_long_url(world: &mut URLShortenerWorld, url: String) -> () {
    world.long_url = url;
}

#[when(expr = "I make a request to the shorten URL endpoint")]
async fn send_shorten_request(world: &mut URLShortenerWorld) -> () {
    let client: Client = Client::new();
    let endpoint = format!(
        "http://localhost:{}/v1/shorten",
        world.container_port.to_string()
    );
    let body = ShortenUrlRequest {
        longUrl: world.long_url.clone(),
    };
    let json_body: serde_json::Value =
        serde_json::to_value(&body).expect("Unable to serialize json");

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
async fn post_shorten_n_times(world: &mut URLShortenerWorld, number_of_requests: u16) -> () {
    let client: Client = Client::new();
    let endpoint = format!(
        "http://localhost:{}/v1/shorten",
        world.container_port.to_string()
    );

    for _ in 0..number_of_requests {
        let body = ShortenUrlRequest {
            longUrl: utility::generate_random_url("http://some_domain"),
        };
        let json_body: serde_json::Value =
            serde_json::to_value(&body).expect("Unable to serialize json");

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

#[when(expr = "I make a request to get all the shortened URLs")]
async fn get_all_shortened_urls(world: &mut URLShortenerWorld) {
    let client: Client = Client::new();
    let endpoint = format!(
        "http://localhost:{}/v1/all",
        world.container_port.to_string()
    );

    let response = client
        .get(&endpoint)
        .send()
        .await
        .expect("Could not send request");
    let status = response.status();
    let response_text = response.text().await.expect("Could not get response text");

    world.get_shortened_url_response =
        serde_json::from_str(&response_text).expect("Could not deserialize json");

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

#[then(expr = "I get {int} values in the get all response")]
async fn get_all_shortened_urls_equals_n_responses(
    world: &mut URLShortenerWorld,
    expected_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(world.get_shortened_url_response.0.len(), expected_count);
    Ok(())
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();
    log::info!("Running feature files");

    // one container is created and closed per scenario
    URLShortenerWorld::cucumber()
        .before(|_feature, _rule, _scenario, _world| {
            Box::pin(async move {
                // todo: can configure the name of the image that you want to run here
                utility::create_and_start_url_shortener_docker_container(
                    _world,
                    "url_shortener_rust",
                    "8080",
                )
                .await;
            })
        })
        .after(|_feature, _rule, _scenario, _ev, _world| {
            Box::pin(async move {
                if let Some(world) = _world {
                    utility::stop_docker_container(&world.container_name).await;
                }
            })
        })
        .run_and_exit("tests/features/url_shortener/url_shortener.feature")
        .await;

    // URLShortenerWorld::run("tests/features/url_shortener/url_shortener.feature").await;
    log::info!("Finished running feature files");
    log::logger().flush();
}

use cucumber::{World as _, given, then, when};
use simple_logger::SimpleLogger;
use url::Url;

mod common;

use common::models::{ShortenUrlRequest, URLShortenerWorld};
use common::*;

#[given(expr = "I have a long URL {string}")]
async fn have_a_long_url(world: &mut URLShortenerWorld, url: String) -> () {
    world.long_url = url;
}

#[when(expr = "I make a request to the shorten URL endpoint")]
async fn send_shorten_request(world: &mut URLShortenerWorld) -> () {
    let endpoint = format!(
        "http://localhost:{}/v1/shorten",
        world.url_shortener_container_host_port.to_string()
    );
    let body = ShortenUrlRequest {
        longUrl: world.long_url.clone(),
    };
    let json_body: serde_json::Value =
        serde_json::to_value(&body).expect("Unable to serialize json");

    let response = world
        .request_client
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
                world.shortened_url_endpoint = text.trim_matches('"').to_string();
            }
            world.shorten_url_status_code = status.as_u16();
        }
        status => {
            log::error!("Request failed! Status: {}", status);
            if let Ok(text) = response.text().await {
                log::error!("Error response: {}", text);
                world.shortened_url_endpoint = text;
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

#[then(expr = "using the returned shortened URL redirects me to {string}")]
async fn make_redirect_request(
    world: &mut URLShortenerWorld,
    expected: String,
) -> Result<(), Box<dyn std::error::Error>> {

    let parsed_url = Url::parse(&world.shortened_url_endpoint).unwrap();
    let path = parsed_url.path().trim_start_matches("/");

    let endpoint = format!(
        "http://localhost:{}/{}",
        world.url_shortener_container_host_port.to_string(),
        path
    );

    let response = world
        .request_client
        .get(endpoint)
        .send()
        .await
        .expect("Could not send request");

    assert_eq!(response.url().as_str(), expected);
    Ok(())
}


#[given(expr = "I make {int} requests to the shorten URL endpoint")]
async fn post_shorten_n_times(world: &mut URLShortenerWorld, number_of_requests: u16) -> () {
    let endpoint = format!(
        "http://localhost:{}/v1/shorten",
        world.url_shortener_container_host_port.to_string()
    );

    for _ in 0..number_of_requests {
        let body = ShortenUrlRequest {
            longUrl: utility::generate_random_url("http://some_domain"),
        };
        let json_body: serde_json::Value =
            serde_json::to_value(&body).expect("Unable to serialize json");

        let response = world
            .request_client
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
    let endpoint = format!(
        "http://localhost:{}/v1/all",
        world.url_shortener_container_host_port.to_string()
    );

    let response = world
        .request_client
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
    assert_eq!(world.get_shortened_url_response.len(), expected_count);
    Ok(())
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();
    log::info!("Running feature files");

    // One container is created and closed per scenario
    URLShortenerWorld::cucumber()
        .before(|_feature, _rule, _scenario, _world| {
            Box::pin(async move {
                // converts async block of code into a future

                // spins up a postgres docker container and a container for the application/microservice that is under test

                let db_user = "postgres";
                let db_pw = "postgres";
                let db_name = "url-shortener-db";

                let env: Option<Vec<String>> = Some(vec![
                    format!("POSTGRES_PASSWORD={}", db_pw),
                    format!("POSTGRES_USER={}", db_user),
                    format!("POSTGRES_DB={}", db_name),
                ]);


                let (postgres_container_name, postgres_container_port) =
                    utility::create_and_start_docker_container(
                        "postgres",
                        "5432",
                        "database system is ready to accept connections",
                        env,
                    )
                    .await;

                _world.db_port = postgres_container_port;
                _world.db_container_name = postgres_container_name;

                let env: Option<Vec<String>> = Some(vec![
                    "DB_HOST=host.docker.internal".to_string(),
                    format!("DB_USER={}", db_user).to_string(),
                    format!("DB_PASSWORD={}", db_pw).to_string(),
                    format!("DB_NAME={}", db_name).to_string(),
                    format!("DB_PORT={}", _world.db_port).to_string(),
                ]);

                let (container_name, container_port) = utility::create_and_start_docker_container(
                    "url_shortener_rust",
                    "8080",
                    "The server has been started",
                    env,
                )
                .await;

                _world.url_shortener_container_name = container_name;
                _world.url_shortener_container_host_port = container_port;
            })
        })
        .after(|_feature, _rule, _scenario, _ev, _world| {
            Box::pin(async move {
                if let Some(world) = _world {
                    utility::stop_docker_container(&world.url_shortener_container_name).await;
                    utility::stop_docker_container(&world.db_container_name).await;
                }
            })
        })
        .run_and_exit("tests/features/url_shortener/url_shortener.feature")
        .await;

    // URLShortenerWorld::run("tests/features/url_shortener/url_shortener.feature").await;
    log::info!("Finished running feature files");
    log::logger().flush();
}

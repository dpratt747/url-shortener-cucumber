use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GetAllShortenUrlResponse {
    long_url: String,
    short_url: String
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ShortenUrlRequest {
    pub longUrl: String,
}

/// [Default] will automatically implement the Default for the following fields
#[derive(cucumber::World, Debug, Default)]
pub struct URLShortenerWorld {
    pub long_url: String,
    pub shorten_url_status_code: u16,
    pub get_shortened_url_response: Vec<GetAllShortenUrlResponse>,
    pub url_shortener_container_host_port: u16,
    pub url_shortener_container_name: String,
    pub request_client: reqwest::Client,
    pub db_port: u16,
    pub db_container_name: String,
}

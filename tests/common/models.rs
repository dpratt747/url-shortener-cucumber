use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GetAllShortenUrlResponse(pub HashMap<String, String>);

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ShortenUrlRequest {
    pub longUrl: String,
}

#[derive(cucumber::World, Debug, Default)]
pub struct URLShortenerWorld {
    pub long_url: String,
    pub shorten_url_status_code: u16,
    pub get_shortened_url_response: GetAllShortenUrlResponse,
    pub container_port: u16,
    pub container_name: String,
}

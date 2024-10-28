use reqwest::Client;
use serde_json::json;
use std::error::Error;

// TODO: perhaps request permission beforehand?
pub async fn get_anki_media_directory() -> Result<String, Box<dyn Error>> {
    const ANKICONNECT_URL: &str = "http://127.0.0.1:8765";

    let request_body = json!({
        "action": "getMediaDirPath",
        "version": 6
    });

    let client = Client::new();
    let response = client
        .post(ANKICONNECT_URL)
        .json(&request_body)
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    if let Some(error) = result["error"].as_str() {
        Err(format!("AnkiConnect Error: {}", error).into())
    } else {
        Ok(result["result"].as_str().unwrap_or("").to_string())
    }
}

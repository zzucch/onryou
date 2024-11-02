use std::error::Error;

pub async fn get_media_directory(ankiconnect_url: &str) -> Result<String, Box<dyn Error>> {
    let request_body = serde_json::json!({
        "action": "getMediaDirPath",
        "version": 6
    });

    let client = reqwest::Client::new();
    let response = client
        .post(ankiconnect_url)
        .json(&request_body)
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    if let Some(error) = result["error"].as_str() {
        Err(format!("ankiconnect error: {error}").into())
    } else {
        Ok(result["result"].as_str().unwrap_or("").to_string())
    }
}

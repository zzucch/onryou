use anyhow::bail;

pub async fn get_media_directory(ankiconnect_url: &str) -> anyhow::Result<String> {
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
        bail!("ankiconnect error: {error}")
    }

    let value = result["result"].as_str();
    match value {
        Some(value) => Ok(value.to_string()),
        None => bail!(
            "expected anki media directory path in 'result' field, \
                but got none"
        ),
    }
}

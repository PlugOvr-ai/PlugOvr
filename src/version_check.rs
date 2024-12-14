use reqwest::blocking::Client;
use serde_json::json;
use std::error::Error;

const LAMBDA_UPDATE_CHECK: &str = "https://5cy8qwxk18.execute-api.eu-central-1.amazonaws.com/v1";

pub fn update_check() -> Result<String, Box<dyn Error>> {
    let client = Client::new();

    let body = json!({
        "version": env!("CARGO_PKG_VERSION"),
    });

    let response = client.post(LAMBDA_UPDATE_CHECK).json(&body).send()?;
    if response.status().is_success() {
        let response_body: serde_json::Value = response.json()?;

        let msg = response_body["body"]
            .as_str()
            .ok_or("Invalid response format")?
            .to_string();
        Ok(msg)
    } else {
        Ok(String::new())
    }
}

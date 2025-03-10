use serde_json::Value;

// const YOUTUBE_API_KEY: &str = include_str!("../youtube_api_key");

pub async fn get_video_title(id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let api_key = std::env::var("YOUTUBE_API_KEY")?;
    let url = format!(
        "https://www.googleapis.com/youtube/v3/videos?key={}&id={}&part=snippet&hl=en",
        api_key, id
    );

    let response = client.get(&url).send().await?; // Send the HTTP GET request

    // Check if the response was successful
    if !response.status().is_success() {
        println!("API request failed with status: {}", response.status());
        println!("Response body: {}", response.text().await?);
        return Err("API request failed".into());
    }

    let json: Value = response.json().await?; // Parse the response body as JSON

    // Check for API errors
    if let Some(error) = json.get("error") {
        print!("API returned an error: {:?}", error);
        return Err("API returned an error".into());
    }

    // Extract video items and add to the videos vector
    if let Some(items) = json["items"].as_array() {
        return Ok(
            items.first().unwrap()["snippet"].as_object().unwrap()["title"]
                .as_str()
                .unwrap()
                .to_string(),
        );
    }

    Err("Could not get video title".into())
}

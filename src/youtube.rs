use serde::Deserialize;

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

    let res: YoutubeVideo = response.json().await?; // Parse the response body as JSON

    // Check for API errors
    if let Some(error) = res.error {
        print!("API returned an error: {:?}", error.message);
        return Err("API returned an error".into());
    }

    // Extract video items and add to the videos vector
    Ok(res.items.first().unwrap().snippet.title.clone())
}

#[derive(Deserialize)]
struct YoutubeError {
    pub message: String,
}

#[derive(Deserialize)]
struct YoutubeSearch {
    pub items: Vec<YoutubeSearchItem>,
    pub error: Option<YoutubeError>,
}

#[derive(Deserialize)]
pub struct YoutubeSearchItem {
    pub id: YoutubeSearchID,
    pub snippet: YoutubeSnippet,
}

#[derive(Deserialize)]
pub struct YoutubeSearchID {
    pub kind: String,
    #[serde(rename = "videoId")]
    pub videoid: Option<String>,
}

#[derive(Deserialize)]
struct YoutubeVideo {
    pub items: Vec<YoutubeVideoItem>,
    pub error: Option<YoutubeError>,
}

#[derive(Deserialize)]
pub struct YoutubeVideoItem {
    pub id: String,
    pub snippet: YoutubeSnippet,
}

#[derive(Deserialize)]
pub struct YoutubeSnippet {
    pub title: String,
}

pub async fn search_videos(
    query: &str,
) -> Result<Vec<YoutubeSearchItem>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let api_key = std::env::var("YOUTUBE_API_KEY")?;
    let url = format!(
        "https://www.googleapis.com/youtube/v3/search?key={}&q={}&part=id,snippet&hl=en&type=video",
        api_key, query
    );

    let response = client.get(&url).send().await?; // Send the HTTP GET request

    // Check if the response was successful
    if !response.status().is_success() {
        println!("API request failed with status: {}", response.status());
        println!("Response body: {}", response.text().await?);
        return Err("API request failed".into());
    }
    // let t = response.text().await?;
    // info!("{}", t);
    let res: YoutubeSearch = response.json().await?; // serde_json::from_str(&t)?;

    // Check for API errors
    if let Some(error) = res.error {
        print!("API returned an error: {:?}", error.message);
        return Err("API returned an error".into());
    }

    Ok(res.items)
}

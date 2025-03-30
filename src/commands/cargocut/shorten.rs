use serde::Deserialize;
use serde::Serialize;
use serenity::all::{
    CommandOptionType, CreateCommand, CreateCommandOption, ResolvedOption, ResolvedValue,
};

#[derive(Serialize, Deserialize)]
pub struct CreateUrl {
    pub long_url: String,
    pub months_valid: Option<u32>,
    pub custom_short_code: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UrlResponse {
    pub short_code: String,
    pub long_url: String,
    pub expiry_date: String,
}

pub async fn shorten(options: &[ResolvedOption<'_>]) -> String {
    // Create a new HTTP client
    let client = reqwest::Client::new();

    // Extract the options using .get() method
    let url = match options.first() {
        Some(ResolvedOption {
            value: ResolvedValue::String(url),
            ..
        }) => url.to_string(),
        _ => return "Please provide a valid URL to shorten.".to_string(),
    };

    let expiry = match options.get(1) {
        Some(ResolvedOption {
            value: ResolvedValue::Number(expiry),
            ..
        }) => Some(*expiry as u32),
        _ => None,
    };

    let custom_url = match options.get(2) {
        Some(ResolvedOption {
            value: ResolvedValue::String(custom_url),
            ..
        }) => Some(custom_url.to_string()),
        _ => None,
    };

    // Check if URL is provided
    if url.is_empty() {
        return "Please provide a URL to shorten.".to_string();
    }

    // Create the request body
    let create_url = CreateUrl {
        long_url: url,
        months_valid: expiry,
        custom_short_code: custom_url,
    };

    // Send the request to the API
    match client
        .post("https://groti.me/api/urls")
        .json(&create_url)
        .send()
        .await
    {
        Ok(response) => {
            match response.status() {
                reqwest::StatusCode::OK | reqwest::StatusCode::CREATED => {
                    match response.json::<UrlResponse>().await {
                        Ok(url_response) => {
                            // Format the response
                            format!(
                                    "URL shortened successfully!\nShort URL: https://groti.me/{}\nExpires on: {}", 
                                    url_response.short_code,
                                    url_response.expiry_date
                                )
                        }
                        Err(_) => "Successfully shortened URL, but couldn't parse the response."
                            .to_string(),
                    }
                }
                reqwest::StatusCode::BAD_REQUEST => {
                    "Bad request. Please check your URL and try again.".to_string()
                }
                reqwest::StatusCode::NOT_FOUND => {
                    "This custom URL is already taken. Please try another one.".to_string()
                }
                _ => format!("Error: HTTP status {}", response.status()),
            }
        }
        Err(e) => format!("Failed to contact the URL shortening service: {}", e),
    }
}

pub fn register_cut() -> CreateCommand {
    CreateCommand::new("cargocut")
        .description("shorten your urls")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "url", "Enter the url to shorten")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Number,
                "expiry",
                "Enter the expiry time in months(default 1 month)",
            )
            .required(false),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "custom_url",
                "enter a short code for your url (groti.me/<your-short-code>)",
            )
            .required(false),
        )
}

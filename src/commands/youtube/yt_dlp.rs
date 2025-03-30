use rand::Rng;
use regex::Regex;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateAttachment, CreateCommand,
    CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage, EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use serenity::Result as SerenityResult;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tokio::time::{sleep, Duration};
use tracing::log::{error, info};

/// Registers the utubeMP3 command with Discord
pub fn register_youtube() -> CreateCommand {
    CreateCommand::new("utubemp3")
        .description("Download MP3 audio from YouTube URL")
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "url", "Enter a YouTube URL")
                .required(true),
        )
}

/// Handles the YouTube MP3 download and sends the file to the user
pub async fn download_mp3(
    ctx: &Context,
    interaction: &CommandInteraction,
    options: &[ResolvedOption<'_>],
) -> SerenityResult<()> {
    info!("Starting YouTube MP3 download process");

    // Extract the YouTube URL
    let youtube_url = match options.first() {
        Some(ResolvedOption {
            value: ResolvedValue::String(url),
            ..
        }) => {
            info!("Received YouTube URL: {}", url);
            url.to_string()
        }
        _ => {
            error!("No valid YouTube URL provided");
            let response = CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Please provide a valid YouTube URL."),
            );
            interaction.create_response(&ctx.http, response).await?;
            return Ok(());
        }
    };

    // Validate the URL is from YouTube
    info!("Validating YouTube URL");
    if !is_valid_youtube_url(&youtube_url) {
        error!("Invalid YouTube URL provided: {}", youtube_url);
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("Please provide a valid YouTube URL (youtube.com or youtu.be)."),
        );
        interaction.create_response(&ctx.http, response).await?;
        return Ok(());
    }

    // Respond to let the user know we're processing
    info!("Sending initial processing message to user");
    let response = CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content("🎵 Processing your YouTube MP3 download request. This may take a moment..."),
    );
    interaction.create_response(&ctx.http, response).await?;

    // Generate a random temp directory name to avoid conflicts
    let random_id: u32 = rand::thread_rng().gen();
    let temp_dir = format!("temp_download_{}", random_id);
    info!("Created temporary directory: {}", temp_dir);

    // Create the temporary directory if it doesn't exist
    if !Path::new(&temp_dir).exists() {
        info!("Creating temporary directory on disk");
        match fs::create_dir(&temp_dir) {
            Ok(_) => info!("Successfully created temp directory"),
            Err(e) => {
                error!("Failed to create temporary directory: {}", e);
                interaction
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content("❌ Failed to create temporary download directory."),
                    )
                    .await?;
                return Ok(());
            }
        }
    }

    // Path for the download
    let output_template = format!("{}/%(title)s.%(ext)s", temp_dir);
    info!("Output template: {}", output_template);

    // Run yt-dlp command to download best audio quality
    info!("Executing yt-dlp command for URL: {}", youtube_url);
    let output = Command::new("yt-dlp")
        .arg("-x") // Extract audio
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("0") // Best quality
        .arg("-o")
        .arg(&output_template)
        .arg(&youtube_url)
        .output();

    // info!("Command to execute: {:?}", command);
    // let output = command.output();

    match output {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("Error running yt-dlp: {}", stderr);

                info!("Sending error message to user");
                let error_msg = interaction.edit_response(
                    &ctx.http,
                    EditInteractionResponse::new().content("❌ Failed to download the YouTube audio. Please check the URL and try again.")
                ).await;

                if let Err(e) = error_msg {
                    error!("Failed to send error message: {}", e);
                }

                // Clean up the temp directory
                info!("Cleaning up temporary directory after error");
                match fs::remove_dir_all(&temp_dir) {
                    Ok(_) => info!("Successfully cleaned up temp directory"),
                    Err(e) => error!("Failed to clean up temp directory: {}", e),
                }
                return Ok(());
            }

            info!("yt-dlp command executed successfully");

            // Find the downloaded MP3 file
            info!("Searching for MP3 file in temp directory");
            let paths = match fs::read_dir(&temp_dir) {
                Ok(paths) => paths,
                Err(e) => {
                    error!("Failed to read temp directory: {}", e);
                    interaction
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new()
                                .content("❌ Failed to locate the downloaded file."),
                        )
                        .await?;

                    // Cleanup attempt
                    let _ = fs::remove_dir_all(&temp_dir);
                    return Ok(());
                }
            };

            let mut mp3_path: Option<PathBuf> = None;

            for path in paths {
                if let Ok(entry) = path {
                    let path = entry.path();
                    info!("Found file in temp dir: {:?}", path);
                    if let Some(extension) = path.extension() {
                        if extension == "mp3" {
                            info!("Found MP3 file: {:?}", path);
                            mp3_path = Some(path);
                            break;
                        }
                    }
                }
            }

            // Send the file if found
            if let Some(file_path) = mp3_path {
                let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
                info!("Preparing to send MP3 file: {}", file_name);

                // Edit the response to indicate the file is ready
                info!("Updating user message - file is ready");
                interaction
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content("✅ Download complete! Sending your MP3 file..."),
                    )
                    .await?;

                // Small delay to ensure the message is updated before sending the file
                info!("Waiting briefly before sending file");
                sleep(Duration::from_millis(500)).await;

                // Read the file
                info!("Reading file content from disk");
                let file_content = match fs::read(&file_path) {
                    Ok(content) => {
                        info!("Successfully read file, size: {} bytes", content.len());
                        content
                    }
                    Err(e) => {
                        error!("Failed to read MP3 file: {}", e);
                        interaction
                            .edit_response(
                                &ctx.http,
                                EditInteractionResponse::new()
                                    .content("❌ Failed to read the MP3 file from disk."),
                            )
                            .await?;

                        // Cleanup
                        let _ = fs::remove_dir_all(&temp_dir);
                        return Ok(());
                    }
                };

                // Create the attachment and send it
                info!("Creating Discord attachment");
                let attachment = CreateAttachment::bytes(file_content, file_name.clone());

                // Check file size
                let file_size = Path::new(&file_path)
                    .metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);
                info!(
                    "File size: {} bytes ({:.2} MB)",
                    file_size,
                    file_size as f64 / 1_048_576.0
                );

                if file_size > 8_388_608 {
                    // 8MB Discord limit
                    tracing::warn!("File exceeds Discord's 8MB size limit");
                    interaction
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new().content(
                                "❌ The file is too large to send via Discord (limit is 8MB).",
                            ),
                        )
                        .await?;

                    // Cleanup
                    let _ = fs::remove_dir_all(&temp_dir);
                    return Ok(());
                }

                // Send the file as a follow-up message
                info!("Sending file attachment to user");
                if let Err(e) = interaction
                    .create_followup(
                        &ctx.http,
                        CreateInteractionResponseFollowup::new()
                            .content("🎵 Enjoy your audio!")
                            .add_file(attachment),
                    )
                    .await
                {
                    error!("Failed to send MP3 file: {}", e);

                    interaction.edit_response(
                        &ctx.http,
                        EditInteractionResponse::new().content("❌ Failed to send the MP3 file. The file might be too large for Discord.")
                    ).await?;
                } else {
                    info!("MP3 file successfully sent to user");
                }
            } else {
                error!("No MP3 file found in the output directory");

                interaction
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content("❌ Failed to find the downloaded MP3 file."),
                    )
                    .await?;
            }

            // Clean up the temp directory
            info!("Cleaning up temporary directory");
            match fs::remove_dir_all(&temp_dir) {
                Ok(_) => info!("Successfully cleaned up temp directory"),
                Err(e) => error!("Failed to clean up temp directory: {}", e),
            }
        }
        Err(e) => {
            error!("Failed to execute yt-dlp command: {}", e);

            interaction.edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("❌ An error occurred while trying to download the audio. Please try again later.")
            ).await?;

            // Clean up the temp directory
            info!("Cleaning up temporary directory after command error");
            match fs::remove_dir_all(&temp_dir) {
                Ok(_) => info!("Successfully cleaned up temp directory"),
                Err(e) => error!("Failed to clean up temp directory: {}", e),
            }
        }
    }

    info!("YouTube MP3 download process completed");
    Ok(())
}

/// Validates that the URL is from YouTube
fn is_valid_youtube_url(url: &str) -> bool {
    let youtube_regex = Regex::new(r"^(https?://)?(www\.)?(youtube\.com|youtu\.?be)/.+$").unwrap();
    youtube_regex.is_match(url)
}

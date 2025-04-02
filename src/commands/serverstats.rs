use serenity::all::{
    CommandInteraction, CreateInteractionResponseMessage, GuildId, OnlineStatus,
};
use serenity::builder::{CreateCommand, EditMessage};
use serenity::http::Http;
use serenity::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

pub fn register() -> CreateCommand {
    CreateCommand::new("serverstats")
        .description("Display real-time server statistics (Total members, online members, active voice users)")
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> String {
    let guild_id = match command.guild_id {
        Some(id) => id,
        None => return "This command can only be used in a guild.".to_string(),
    };

    
    let stats = fetch_stats(ctx, guild_id).await;

    
    if let Err(why) = command
        .create_response(
            &ctx.http,
            serenity::builder::CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(stats.clone()),
            ),
        )
        .await
    {
        eprintln!("Failed to create response: {:?}", why);
        return "Failed to create response".to_string();
    }

    
    if let Ok(message) = command.get_response(&ctx.http).await {
        let update_interval: u64 = 30;
        let http = ctx.http.clone();
        tokio::spawn(async move {
            let mut msg = message;
            loop {
                sleep(Duration::from_secs(update_interval)).await;
                let new_stats = fetch_stats_from_http(&http, guild_id).await;
                let builder = EditMessage::new().content(new_stats);
                if let Err(e) = msg.edit(&http, builder).await {
                    eprintln!("Error editing server stats message: {:?}", e);
                    break;
                }
            }
        });
    }

    stats
}


pub async fn fetch_stats(ctx: &Context, guild_id: GuildId) -> String {
    if let Some(guild_lock) = ctx.cache.guild(guild_id) {
        
        let guild = &*guild_lock;
        let total_members = guild.members.len();

        
        let online_members = guild
            .presences
            .values()
            .filter(|presence| {
                matches!(
                    presence.status,
                    OnlineStatus::Online | OnlineStatus::Idle | OnlineStatus::DoNotDisturb
                )
            })
            .count();

        let active_voice_users = guild
            .voice_states
            .values()
            .filter(|vs| vs.channel_id.is_some())
            .count();

        format!(
            "**Server Stats:**\nTotal Members: {}\nOnline Members: {}\nActive in Voice: {}",
            total_members, online_members, active_voice_users
        )
    } else {
        "Error: Guild not found in cache.".to_string()
    }
}

pub async fn fetch_stats_from_http(http: &Http, guild_id: GuildId) -> String {
    if let Ok(guild) = guild_id.to_partial_guild(http).await {
        let total_members = guild.approximate_member_count.unwrap_or(0);
        format!(
            "**Server Stats (HTTP):**\nTotal Members: {}\nOnline Members: {}\nActive in Voice: {}",
            total_members, 0, 0
        )
    } else {
        "Error: Unable to fetch guild info.".to_string()
    }
}

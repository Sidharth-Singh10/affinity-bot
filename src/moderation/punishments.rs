use crate::moderation::violations::{ModAction, ViolationsTracker};
use serenity::all::{
    Message, Timestamp,
};
use serenity::prelude::*;
use std::error::Error;
use tracing::error;

pub async fn punish_member(
    ctx: &Context,
    msg: &Message,
    action: Option<ModAction>,
    violations_tracker: &ViolationsTracker,
) -> Result<(), Box<dyn Error>> {
    match action {
        Some(ModAction::Mute(duration)) => {
            if let Some(guild_id) = msg.guild_id {
                if let Ok(mut member) = guild_id.member(&ctx.http, msg.author.id).await {
                    let until = Timestamp::from_unix_timestamp(
                        (std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)? 
                            .as_secs() as i64) 
                        + duration.as_secs() as i64,
                    )?;
                    if let Err(e) = member
                        .disable_communication_until_datetime(&ctx.http, until)
                        .await
                    {
                        error!("Failed to timeout member: {:?}", e);
                    }
                    msg.channel_id
                        .say(
                            &ctx.http,
                            format!(
                                "User {} has been muted for {} violations",
                                msg.author.mention(),
                                violations_tracker
                                    .get_violation_count(msg.author.id)
                                    .unwrap()
                            ),
                        )
                        .await?;
                }
            }
        }
        Some(ModAction::Ban) => {
            msg.guild_id
                .ok_or("Not in guild")?
                .ban_with_reason(
                    &ctx.http,
                    msg.author.id,
                    7, // Delete messages from last 7 days
                    "Exceeded violation limit",
                )
                .await?;
            msg.channel_id
                .say(
                    &ctx.http,
                    format!(
                        "User {} has been banned for excessive violations",
                        msg.author.name
                    ),
                )
                .await?;
        }
        Some(ModAction::None) => {
            let warning_message = format!(
                "You're sending messages too quickly {}. Please slow down to avoid being timed out.",
                msg.author.id.mention()
            );

            msg.channel_id.say(&ctx.http, warning_message).await?;
        }
        None => {
            msg.channel_id
                .say(&ctx.http, "Error checking violations")
                .await?;
        }
    }

    Ok(())
}

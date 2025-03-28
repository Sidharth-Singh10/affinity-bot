use serenity::all::{
    Color, CommandInteraction, Context, CreateEmbed, CreateEmbedFooter, Mentionable, Timestamp,
};
use serenity::builder::{CreateCommand, CreateCommandOption};
use serenity::model::application::{CommandOptionType, ResolvedOption, ResolvedValue};
use tracing::error;

pub fn warn(options: &[ResolvedOption]) -> String {
    if let (
        Some(ResolvedOption {
            value: ResolvedValue::User(user, _),
            ..
        }),
        Some(ResolvedOption {
            value: ResolvedValue::String(reason),
            ..
        }),
    ) = (options.first(), options.get(1))
    {
        format!(
            "**Warning** issued to {} **for: {}**",
            user.mention(),
            reason
        )
    } else {
        "Please provide a valid user and reason".to_string()
    }
}

/// Mutes a guild member for a specified duration (in minutes). Optionally, a reason may be provided.
/// The command updates the guild member's timeout and returns an embed containing details.
///
/// Expects options:
///   - 0: User (required)
///   - 1: Duration in minutes (required)
///   - 2: Reason (optional)
pub async fn mute(
    options: &[ResolvedOption<'_>],
    ctx: &Context,
    command: &CommandInteraction,
) -> CreateEmbed {
    if let (
        Some(ResolvedOption {
            value: ResolvedValue::User(user, _),
            ..
        }),
        Some(ResolvedOption {
            value: ResolvedValue::Integer(time),
            ..
        }),
    ) = (options.first(), options.get(1))
    {
        // Optional reason; default to an empty string if not provided.
        let reason = if let Some(ResolvedOption {
            value: ResolvedValue::String(r),
            ..
        }) = options.get(2)
        {
            r
        } else {
            ""
        };

        let until = Timestamp::from_unix_timestamp(
            (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64)
                + time * 60,
        )
        .unwrap();

        // Get the guild ID from the command's guild_id field
        let guild_id = match command.guild_id {
            Some(id) => id,
            None => {
                return CreateEmbed::default()
                    .title("Error")
                    .description("Failed to fetch guild ID")
                    .color(Color::RED)
            }
        };

        let mut guild_member = match guild_id.member(&ctx.http, user.id).await {
            Ok(member) => member,
            Err(_) => {
                return CreateEmbed::default()
                    .title("Error")
                    .description("Failed to fetch guild member")
                    .color(Color::RED)
            }
        };

        if let Err(e) = guild_member
            .disable_communication_until_datetime(&ctx.http, until)
            .await
        {
            error!("Failed to timeout member: {:?}", e);
            return CreateEmbed::default()
                .title("Error")
                .description("Failed to mute user")
                .color(Color::RED);
        }

        let mut embed = CreateEmbed::default()
            .title("User Muted")
            .description(format!("**{}** has been muted", user.tag()))
            .field("User ID", user.id.to_string(), true)
            .field("Duration", format!("{} minutes", time), true)
            .color(Color::RED)
            .footer(CreateEmbedFooter::new("Muted by Moderation System"));

        if !reason.is_empty() {
            embed = embed.field("Reason", reason, false);
        }

        embed
    } else {
        CreateEmbed::default()
            .title("Error")
            .description("User is not a member of this server")
            .color(Color::RED)
    }
}

/// Bans a guild member. Optionally, a reason may be provided.
/// The command also deletes messages from the past X days (provided as an integer).
///
/// Expects options:
///   - 0: User (required)
///   - 1: Days of messages to delete (required)
///   - 2: Reason (optional)
pub async fn ban(
    options: &[ResolvedOption<'_>],
    ctx: &Context,
    command: &CommandInteraction,
) -> CreateEmbed {
    if let (
        Some(ResolvedOption {
            value: ResolvedValue::User(user, _),
            ..
        }),
        Some(ResolvedOption {
            value: ResolvedValue::Integer(days),
            ..
        }),
    ) = (options.first(), options.get(1))
    {
        let reason = if let Some(ResolvedOption {
            value: ResolvedValue::String(r),
            ..
        }) = options.get(2)
        {
            r
        } else {
            ""
        };

        let guild_id = match command.guild_id {
            Some(id) => id,
            None => {
                return CreateEmbed::default()
                    .title("Error")
                    .description("Failed to fetch guild ID")
                    .color(Color::RED)
            }
        };

        let guild_member = match guild_id.member(&ctx.http, user.id).await {
            Ok(member) => member,
            Err(_) => {
                return CreateEmbed::default()
                    .title("Error")
                    .description("Failed to fetch guild member")
                    .color(Color::RED)
            }
        };

        if let Err(e) = guild_member.ban(&ctx.http, *days as u8).await {
            error!("Failed to timeout member: {:?}", e);
            return CreateEmbed::default()
                .title("Error")
                .description("Failed to mute user")
                .color(Color::RED);
        }

        let mut embed = CreateEmbed::default()
            .title("User Banned")
            .description(format!("**{}** has been banned", user.tag()))
            .field("User ID", user.id.to_string(), true)
            .color(Color::RED)
            .footer(CreateEmbedFooter::new("banned by Moderation System"));

        if !reason.is_empty() {
            embed = embed.field("Reason", reason, false);
        }

        embed
    } else {
        CreateEmbed::default()
            .title("Error")
            .description("User is not a member of this server")
            .color(Color::RED)
    }
}

pub fn register_warn() -> CreateCommand {
    CreateCommand::new("warn")
        .description("warn a member")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "id", "The user to lookup")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "reason",
                "The reason for the warning",
            )
            .required(true),
        )
}

pub fn register_mute() -> CreateCommand {
    CreateCommand::new("mute")
        .description("mute a member")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "id", "The user to lookup")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "duration",
                "duration in minutes to be muted",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "reason",
                "Optional reason for muting the user",
            )
            .required(false),
        )
}

pub fn register_ban() -> CreateCommand {
    CreateCommand::new("ban")
        .description("ban a member")
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "id", "The user to lookup")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "days",
                "Number of days worth of messages to be deleted",
            )
            .required(true),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "reason",
                "Optional reason for banning the user",
            )
            .required(false),
        )
}

use anyhow::Context as _;
use config::email::EmailConfig;
use cron::notifications::{self, NotificationManager};
use events::self_role_assign::self_role_assign;
use moderation::spam::SpamChecker;
use moderation::violations::{ViolationThresholds, ViolationsTracker};
use scraper::price_scraper::PriceScraper;
use sea_orm::{Database, DatabaseConnection};
use serenity::all::{
    ChannelId, Command, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId,
    Interaction, Member, Message, MessageId, Reaction, ReactionType, Ready, User,
};
use serenity::async_trait;
use serenity::prelude::*;
use shuttle_runtime::SecretStore;
use std::error::Error;
use tokio::time::Instant;
use tracing::{error, info};
use moderation::punishments::punish_member;
mod commands;
mod config;
mod cron;
mod events;
mod moderation;
mod scraper;
mod utils;
use std::time::Duration;
struct Bot {
    secrets: SecretStore,
    spam_checker: SpamChecker,
    violations_tracker: ViolationsTracker,
    violation_threshold: ViolationThresholds,
    db: DatabaseConnection,
}

impl Bot {
    pub fn new(secrets: SecretStore, db: DatabaseConnection) -> Self {
        Self {
            secrets,
            spam_checker: SpamChecker::new(),
            violations_tracker: ViolationsTracker::new(),
            violation_threshold: ViolationThresholds::default(),
            db,
        }
    }
    async fn reaction_add_internal(
        &self,
        ctx: Context,
        reaction: Reaction,
    ) -> Result<(), Box<dyn Error>> {
        // Parse the role_message_id
        let role_message_id = match self
            .secrets
            .get("ROLE_EMOJI_MESSAGE_ID")
            .ok_or("Missing ROLE_EMOJI_MESSAGE_ID")?
            .parse()
        {
            Ok(id) => MessageId::new(id),
            Err(e) => {
                error!("Failed to parse ROLE_EMOJI_MESSAGE_ID: {:?}", e);
                return Err(Box::new(e));
            }
        };

        println!("hello i am here");

        // Check if the reaction matches the role message
        if reaction.message_id == role_message_id {
            self_role_assign(ctx, reaction, self.secrets.clone()).await?;
        }

        Ok(())
    }

    async fn new_members(&self, ctx: Context, new_member: Member) -> Result<(), Box<dyn Error>> {
        events::onboarding_role::new_member_role_assign(ctx, new_member, self.secrets.clone())
            .await?;

        Ok(())
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, ctx: Context, msg: Message) {
        // Ignore bot messages
        if msg.author.bot {
            return;
        }
        // Check for spam
        if self.spam_checker.is_spam(msg.author.id).await {
            // Delete spam message
            println!("{} is spamming ", msg.author.id);
            if let Err(e) = msg.delete(&ctx.http).await {
                error!("Failed to delete spam message: {:?}", e);
            }

            // Unused Result (Scope of Improvement)
            let _ = self.violations_tracker.increment_violations(msg.author.id);
            let action = self
                .violations_tracker
                .get_appropriate_action(msg.author.id, &self.violation_threshold);

            // Unused Result (Scope of Improvement)
            let _ = punish_member(&ctx, &msg, action, &self.violations_tracker).await;

            return;
        }

        if msg.content == "!hello" {
            if let Err(e) = msg.channel_id.say(&ctx.http, "world!").await {
                error!("Error sending message: {:?}", e);
            }
        }
    }
    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let welcome_channel_id = match self
            .secrets
            .get("WELCOME_CHANNEL_ID")
            .expect("Set WELCOME_CHANNEL_ID")
            .parse()
        {
            Ok(id) => ChannelId::new(id),
            Err(e) => {
                error!("Failed to parse WELCOME_CHANNEL_ID: {:?}", e);
                return;
            }
        };

        let welcome_message = format!("Welcome to the server, {}! 👋", new_member.mention());

        if let Err(e) = welcome_channel_id.say(&ctx.http, welcome_message).await {
            error!("Error sending welcome message: {:?}", e);
        }
        if let Err(e) = self.new_members(ctx, new_member).await {
            error!("Error handling new_members: {:?}", e);
        }
    }
    async fn guild_member_removal(&self, ctx: Context, _: GuildId, user: User, _: Option<Member>) {
        let welcome_channel_id = match self
            .secrets
            .get("WELCOME_CHANNEL_ID")
            .expect("Set WELCOME_CHANNEL_ID")
            .parse()
        {
            Ok(id) => ChannelId::new(id),
            Err(e) => {
                error!("Failed to parse WELCOME_CHANNEL_ID: {:?}", e);
                return;
            }
        };

        let goodbye_message = format!("Will Miss You {}! 👋", user.mention());
        if let Err(e) = welcome_channel_id.say(&ctx.http, goodbye_message).await {
            error!("Error sending welcome message: {:?}", e);
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if let Err(e) = self.reaction_add_internal(ctx, reaction).await {
            error!("Error handling reaction_add: {:?}", e);
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            // println!("Received command interaction: {command:#?}");

            match command.data.name.as_str() {
                "ping" => {
                    utils::util::create_response(
                        &ctx,
                        &command,
                        commands::ping::run(&command.data.options()),
                    )
                    .await
                }
                "id" => {
                    utils::util::create_response(
                        &ctx,
                        &command,
                        commands::id::run(&command.data.options()),
                    )
                    .await
                }
                "welcome" => {
                    // Handle the welcome command
                    let message = commands::welcome_message::run(&command.data.options());
                    let data = CreateInteractionResponseMessage::new().content(message);
                    let builder = CreateInteractionResponse::Message(data);
                    if let Err(why) = command.create_response(&ctx.http, builder).await {
                        error!("Cannot respond to welcome command: {why}");
                    } else {
                        // Get the message after sending it
                        if let Ok(message) = command.get_response(&ctx.http).await {
                            // Add reactions one by one
                            let reactions = [
                                // Backgrounds
                                "🦀", "🕸️", "🧠", "🫖",
                            ];

                            for reaction in reactions {
                                // Add each reaction with a small delay to avoid rate limiting
                                if let Err(e) = message
                                    .react(&ctx.http, ReactionType::Unicode(reaction.to_string()))
                                    .await
                                {
                                    error!("Error adding reaction {}: {:?}", reaction, e);
                                }
                                // Small delay between reactions to avoid rate limits
                                tokio::time::sleep(Duration::from_millis(300)).await;
                            }
                        }
                    }
                }
                "warn" => {
                    utils::util::create_response(
                        &ctx,
                        &command,
                        commands::moderate::warn(&command.data.options()),
                    )
                    .await
                }
                "mute" => {
                    utils::util::create_response(
                        &ctx,
                        &command,
                        commands::moderate::mute(&command.data.options(), &ctx, &command).await,
                    )
                    .await
                }
                "ban" => {
                    utils::util::create_response(
                        &ctx,
                        &command,
                        commands::moderate::ban(&command.data.options(), &ctx, &command).await,
                    )
                    .await
                }
                "myntra" => {
                    let response =
                        commands::scrape::myntra::myntra_add(&command.data.options(), &self.db)
                            .await
                            .unwrap();
                    let message = CreateInteractionResponseMessage::new().content(response);
                    let builder = CreateInteractionResponse::Message(message);
                    if let Err(why) = command.create_response(&ctx.http, builder).await {
                        error!("Cannot respond to myntra_add command: {why}");
                    }
                }
                "cargocut" => {
                    // Handle URL shortening command

                    let response =
                        commands::cargocut::shorten::shorten(&command.data.options()).await;

                    let message = CreateInteractionResponseMessage::new().content(response);
                    let builder = CreateInteractionResponse::Message(message);
                    if let Err(why) = command.create_response(&ctx.http, builder).await {
                        error!("Cannot respond to myntra_add command: {why}");
                    }
                }
                "utubemp3" => {
                    // Handle the YouTube MP3 download command
                    if let Err(why) =
                        commands::youtube::yt_dlp::download_mp3(&ctx, &command, &command.data.options())
                            .await
                    {
                        error!("Error in utubeMP3 command: {why}");

                        // Send error message if something goes wrong
                        let error_message = CreateInteractionResponseMessage::new()
                        .content("An error occurred while processing your request. Please try again later.");
                        let error_response = CreateInteractionResponse::Message(error_message);

                        if let Err(e) = command.create_response(&ctx.http, error_response).await {
                            error!("Cannot send error response for utubeMP3 command: {e}");
                        }
                    }
                }
                "poll" => {
                    let response = commands::vote::run(&ctx, &command).await;
                    utils::util::create_response(&ctx, &command, response).await;
                }
                _ => {
                    utils::util::create_response(&ctx, &command, "not implemented :(".to_string())
                        .await
                }
            };
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        // cleanUp of spamHashMap entries
        let tracker = self.spam_checker.get_tracker();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let mut guard = tracker.lock().await;
                let now = Instant::now();
                guard.retain(|_, data| {
                    now.duration_since(data.first_message) <= Duration::from_secs(30)
                });
            }
        });

        // Now we can access secrets through self.secrets
        let guild_id = GuildId::new(
            self.secrets
                .get("GUILD_TOKEN")
                .context("secret not set")
                .unwrap()
                .parse()
                .expect("Failed to parse GUILD_TOKEN"),
        );

        let commands = guild_id
            .set_commands(
                &ctx.http,
                vec![
                    commands::scrape::myntra::register_add(),
                    commands::ping::register(),
                    commands::id::register(),
                    commands::welcome_message::register(),
                    commands::moderate::register_warn(),
                    commands::moderate::register_mute(),
                    commands::moderate::register_ban(),
                    commands::cargocut::shorten::register_cut(),
                    commands::youtube::yt_dlp::register_youtube(),
                    commands::vote::register(),
                ],
            )
            .await;
        println!("I now have the following guild slash commands: {commands:#?}");
        let _ = Command::create_global_command(&ctx.http, commands::wonderful_command::register())
            .await;
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    let token = secrets
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let db = Database::connect(secrets.get("DATABASE_URL").unwrap())
        .await
        .expect("could not connect");

    EmailConfig::init(&secrets).expect("Could not initialize email config");

    let scraper = PriceScraper::new(db.clone());
    tokio::spawn(async move { scraper.start_scraping().await });

    let mut manager = NotificationManager::new(db.clone());
    manager.register_handler(notifications::MyntraHandler);
    manager.start().await;

    // Pass secrets to Bot constructor
    let client = Client::builder(&token, intents)
        .event_handler(Bot::new(secrets, db))
        .await
        .expect("Err creating client");

    Ok(client.into())
}

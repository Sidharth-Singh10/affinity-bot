use serenity::all::{
    CommandInteraction, CreateInteractionResponseMessage, ReactionType, CommandOptionType,
    CommandDataOptionValue,
};
use serenity::builder::CreateCommand;
use serenity::prelude::*;
use tokio::time::{sleep, Duration};
use tracing::error;


const DEFAULT_POLL_DURATION: u64 = 60;


const NUMBER_EMOJIS: [&str; 10] = [
    "1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣", "🔟",
];

pub fn register() -> CreateCommand {
    CreateCommand::new("poll")
        .description("Create a poll. Syntax: /poll \"Question\" \"Option1\" \"Option2\" ... [duration in seconds]")
        .add_option(
            serenity::builder::CreateCommandOption::new(
                CommandOptionType::String,
                "question",
                "The poll question",
            )
            .required(true),
        )
        .add_option(
            serenity::builder::CreateCommandOption::new(
                CommandOptionType::String,
                "option1",
                "First option",
            )
            .required(true),
        )
        .add_option(
            serenity::builder::CreateCommandOption::new(
                CommandOptionType::String,
                "option2",
                "Second option",
            )
            .required(true),
        )
        
        .add_option(
            serenity::builder::CreateCommandOption::new(
                CommandOptionType::String,
                "option3",
                "Option 3",
            )
            .required(false),
        )
        .add_option(
            serenity::builder::CreateCommandOption::new(
                CommandOptionType::String,
                "option4",
                "Option 4",
            )
            .required(false),
        )
        
        
        .add_option(
            serenity::builder::CreateCommandOption::new(
                CommandOptionType::Integer,
                "duration",
                "Poll duration in seconds (optional)",
            )
            .required(false),
        )
}

pub async fn run(ctx: &Context, command: &CommandInteraction) -> String {
    
    let mut question = String::new();
    let mut poll_options: Vec<String> = Vec::new();
    let mut poll_duration: u64 = DEFAULT_POLL_DURATION; 

    for opt in &command.data.options {
        match opt.name.as_str() {
            "question" => {
                if let CommandDataOptionValue::String(q) = &opt.value {
                    question = q.clone();
                }
            }
            name if name.starts_with("option") => {
                if let CommandDataOptionValue::String(val) = &opt.value {
                    poll_options.push(val.clone());
                }
            }
            "duration" => {
                if let CommandDataOptionValue::Integer(dur) = &opt.value {
                    
                    poll_duration = *dur as u64;
                }
            }
            _ => {}
        }
    }

    
    if question.is_empty() || poll_options.len() < 2 {
        return "You need a poll question and at least two options.".to_string();
    }
    if poll_options.len() > 10 {
        poll_options.truncate(10);
    }

    
    let mut content = format!("**Poll:** {}\n", question);
    for (i, option) in poll_options.iter().enumerate() {
        let emoji = NUMBER_EMOJIS[i];
        content.push_str(&format!("{} {}\n", emoji, option));
    }
    content.push_str(&format!("\nPoll duration: {} seconds", poll_duration));

    
    if let Err(e) = command.create_response(
        &ctx.http,
        serenity::builder::CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("Poll created!"),
        ),
    )
    .await {
        error!("Error sending poll confirmation: {:?}", e);
    }

    
    let channel_id = command.channel_id;
    let poll_message = match channel_id.say(&ctx.http, content.clone()).await {
        Ok(msg) => msg,
        Err(e) => {
            error!("Error sending poll message: {:?}", e);
            return "Failed to create poll message.".to_string();
        }
    };

    
    for i in 0..poll_options.len() {
        let emoji = NUMBER_EMOJIS[i];
        if let Err(e) = poll_message
            .react(&ctx.http, ReactionType::Unicode(emoji.to_string()))
            .await {
            error!("Error adding reaction {}: {:?}", emoji, e);
        }
    }

    
    let http = ctx.http.clone();
    let poll_message_id = poll_message.id;
    let channel_id_clone = channel_id;
    let poll_options_clone = poll_options.clone();
    tokio::spawn(async move {
        sleep(Duration::from_secs(poll_duration)).await;
        match channel_id_clone.message(&http, poll_message_id).await {
            Ok(msg) => {
                let mut results = String::from("**Poll Results:**\n");
                for (i, option) in poll_options_clone.iter().enumerate() {
                    let emoji = NUMBER_EMOJIS[i];
                    
                    let vote_count = msg.reactions.iter()
                        .find(|r| {
                            r.reaction_type.to_string() ==
                            ReactionType::Unicode(emoji.to_string()).to_string()
                        })
                        .map(|r| if r.count > 1 { r.count - 1 } else { 0 })
                        .unwrap_or(0);
                    results.push_str(&format!("{} {} - {} votes\n", emoji, option, vote_count));
                }
                if let Err(e) = channel_id_clone.say(&http, results).await {
                    error!("Error sending poll results: {:?}", e);
                }
            },
            Err(e) => {
                error!("Error fetching poll message for tally: {:?}", e);
            }
        }
    });

    "Poll created successfully!".to_string()
}

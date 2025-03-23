use std::{collections::HashMap, sync::Arc};

use log::{error, info};
use serenity::{
    all::GuildId,
    async_trait,
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    model::{
        application::{Command, Interaction},
        gateway::Ready,
    },
    prelude::*,
};

mod commands;
pub mod youtube;

// const BOT_TOKEN: &str = include_str!("../bot_token");
const COLOR_OK: u32 = 0xcba6f7;
const COLOR_ERROR: u32 = 0xf38ba8;

// TODO: AFK timeout of 10 mins in empty vc

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        let commands = [
            Command::create_global_command(&ctx.http, commands::play::register()).await,
            Command::create_global_command(&ctx.http, commands::r#loop::register()).await,
            Command::create_global_command(&ctx.http, commands::volume::register()).await,
            Command::create_global_command(&ctx.http, commands::stop::register()).await,
            Command::create_global_command(&ctx.http, commands::disconnect::register()).await,
            Command::create_global_command(&ctx.http, commands::pause::register()).await,
            // Command::create_global_command(&ctx.http, commands::record::register()).await,
        ];

        info!("Created {} commands", commands.len());
        // println!("I created the following global slash command: {command:#?}");
    }

    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        info!("Cache is ready!");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            // println!("Received command interaction: {command:#?}");

            match command.data.name.as_str() {
                "play" => {
                    commands::play::run(&ctx, &command).await.unwrap();
                }
                "loop" => {
                    commands::r#loop::run(&ctx, &command).await.unwrap();
                }
                "volume" => {
                    commands::volume::run(&ctx, &command).await.unwrap();
                }
                "stop" => {
                    commands::stop::run(&ctx, &command).await.unwrap();
                }
                "disconnect" => {
                    commands::disconnect::run(&ctx, &command).await.unwrap();
                }
                "pause" => {
                    commands::pause::run(&ctx, &command).await.unwrap();
                }
                // "record" => {
                //     commands::record::run(&ctx, &command).await.unwrap();
                // }
                _ => {}
            };

            // if let Some(content) = content {
            //     let data = CreateInteractionResponseMessage::new().content(content);
            //     let builder = CreateInteractionResponse::Message(data);
            //     if let Err(why) = command.create_response(&ctx.http, builder).await {
            //         println!("Cannot respond to slash command: {why}");
            //     }
            // }
        }
    }
}

use reqwest::Client as HttpClient;
use songbird::{Config, tracks::TrackHandle};

struct UserData {
    http: HttpClient,
    songbird: Arc<songbird::Songbird>,
    track_handles: HashMap<GuildId, TrackHandle>,
}

impl TypeMapKey for UserData {
    type Value = UserData;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(
        env_logger::Env::new()
            .default_filter_or("info,serenity=warn,songbird=warn,tracing=warn,symphonia_core=warn"),
    );

    let manager = songbird::Songbird::serenity();
    let user_data = UserData {
        http: HttpClient::new(),
        songbird: Arc::clone(&manager),
        track_handles: HashMap::new(),
    };

    let token = std::env::var("BOT_TOKEN")?;

    let intents =
        GatewayIntents::default() | GatewayIntents::GUILDS | GatewayIntents::GUILD_VOICE_STATES;
    let mut client = Client::builder(token, intents)
        .type_map_insert::<UserData>(user_data)
        .voice_manager_arc(manager)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    tokio::spawn(async move {
        let _ = client
            .start_autosharded()
            .await
            .map_err(|why| error!("Client ended: {:?}", why));
    });

    let _signal_err = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shutting down.");

    Ok(())
}

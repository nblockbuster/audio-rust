use log::{error, info, warn};
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::{TrackEvent, input::YoutubeDl};
use url::*;

use crate::{COLOR_ERROR, COLOR_OK, UserData, commands::TrackErrorNotifier};

pub fn register() -> CreateCommand {
    CreateCommand::new("play")
        .description("Play a song")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "link",
                "The link of the audio to play",
            )
            .required(true),
        )
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let url = {
        if let Some(ResolvedOption {
            value: ResolvedValue::String(url_str),
            ..
        }) = interaction.data.options().first().cloned()
        {
            let url = Url::parse(url_str);
            if let Err(err) = url {
                interaction
                    .create_response(
                        ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().embed(
                                CreateEmbed::new()
                                    .color(Colour::new(COLOR_ERROR))
                                    .description(format!("Not a valid URL: {}", err))
                                    .title("Error")
                                    .timestamp(Timestamp::now()),
                            ),
                        ),
                    )
                    .await?;
                None
            } else {
                Some(url.unwrap())
            }
        } else {
            None
        }
    };

    if url.is_none() {
        // TODO: search
        warn!("url none");
        return Ok(());
    }

    let url = url.unwrap();

    if !url.to_string().contains("youtu") {
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(
                            CreateEmbed::new()
                                .color(Colour::new(COLOR_ERROR))
                                .description("Not a valid Youtube URL")
                                .title("Error")
                                .timestamp(Timestamp::now()),
                        )
                        .ephemeral(true),
                ),
            )
            .await?;
        warn!("not a valid youtube url");
        return Ok(());
    }

    let (guild_id, channel_id) = {
        let guild_id = interaction.guild_id.unwrap();
        let user = interaction.user.id;

        let g = ctx.cache.guild(guild_id);
        if g.is_none() {
            let _ = interaction.create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .color(Colour::new(COLOR_ERROR))
                            .description("Could not get current guild information")
                            .title("Error")
                            .timestamp(Timestamp::now()),
                    ),
                ),
            );

            // TODO: guild err
            error!("guild none");
            return Ok(());
        }
        let g = g.unwrap();
        // TODO: no voice
        let vs = g.voice_states.get(&user).unwrap();

        (guild_id, vs.channel_id.unwrap())
    };

    info!("{}: {}", guild_id, url);
    let mut typemap = ctx.data.write().await;
    let data = typemap.get_mut::<UserData>().unwrap();
    let manager = &data.songbird;
    if let Some(track) = data.track_handles.get_mut(&guild_id) {
        let _ = track.stop();
    }

    let call = manager.join(guild_id, channel_id).await;
    if let Ok(handler_lock) = call {
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        let _ = handler.deafen(true).await;
    } else if let Err(e) = call {
        warn!("{}", e);
    }

    if let Some(handler_lock) = data.songbird.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let src = YoutubeDl::new(data.http.clone(), url.to_string());
        let song = handler.play_input(src.into());
        // TODO: persist loop setting
        let _ = song.enable_loop();
        let _ = song.set_volume(0.5);

        let mut title = String::new();

        let pairs = url.query_pairs();
        for pair in pairs {
            if pair.0 == "v" {
                if let Ok(title1) = crate::youtube::get_video_title(&pair.1).await {
                    title = title1;
                }
            }
        }

        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .color(Colour::new(COLOR_OK))
                            .title("Now Playing")
                            .description(format!("[{}]({})", title, url))
                            .timestamp(Timestamp::now()),
                    ),
                ),
            )
            .await?;

        data.track_handles.insert(guild_id, song);
    } else {
        // TODO: error
        error!("Songbird get none");
        // check_msg(
        //     msg.channel_id
        //         .say(&ctx.http, "Not in a voice channel to play in")
        //         .await,
        // );
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .color(Colour::new(COLOR_ERROR))
                            .title("Error")
                            .description("Could not get Songbird manager for guild")
                            .timestamp(Timestamp::now()),
                    ),
                ),
            )
            .await?;
    }

    Ok(())
}

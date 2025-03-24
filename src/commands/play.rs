use log::{error, warn};
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
                CommandOptionType::SubCommand,
                "web",
                "The audio to play is a link on the internet",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "link",
                    "The link of the audio",
                )
                .required(true),
            ),
        )
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "file",
                "The audio to play is a local file",
            )
            .add_sub_option(
                CreateCommandOption::new(
                    CommandOptionType::Attachment,
                    "file",
                    "Audio file to play",
                )
                .required(true),
            ),
        )
}

pub async fn run_command(
    ctx: &Context,
    interaction: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let mut filename = String::new();
    let mut final_url: Option<Url> = None;
    let mut search_str = String::new();
    if let Some(ResolvedOption {
        value: ResolvedValue::SubCommand(options),
        ..
    }) = interaction.data.options().first().cloned()
    {
        if let Some(ResolvedOption {
            value: ResolvedValue::String(url_str),
            ..
        }) = options.first().cloned()
        {
            let url = Url::parse(url_str);
            if url.is_err() {
                search_str = url_str.to_string();
            } else {
                final_url = Some(url.unwrap());
            }
        } else if let Some(ResolvedOption {
            value: ResolvedValue::Attachment(a),
            ..
        }) = options.first().cloned()
        {
            filename = a.filename.clone();
            let url = Url::parse(&a.url);
            if let Err(err) = url {
                interaction
                    .create_response(
                        ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().embed(
                                CreateEmbed::new()
                                    .color(Colour::new(COLOR_ERROR))
                                    .description(format!("Not a valid attachment URL: {}", err))
                                    .title("Error")
                                    .timestamp(Timestamp::now()),
                            ),
                        ),
                    )
                    .await?;
            } else {
                final_url = Some(url.unwrap());
            }
        }
    } else {
        warn!("play interaction option not subcommand");
        return Ok(());
    }
    let mut is_search = false;
    if final_url.is_none() {
        is_search = true;
        // warn!("url none");
    }

    if is_search {
        let results = crate::youtube::search_videos(&search_str).await.unwrap();
        let mut menu_options = vec![];
        results[..5].iter().for_each(|x| {
            if x.id.videoid.is_none() || x.id.videoid.is_none() {
                return;
            }
            menu_options.push(CreateSelectMenuOption::new(
                truncate(x.snippet.title.as_str(), 100),
                x.id.videoid.clone().unwrap(),
            ));
        });

        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().select_menu(
                        CreateSelectMenu::new(
                            "select_search",
                            CreateSelectMenuKind::String {
                                options: menu_options,
                            },
                        )
                        .placeholder("Select a video"),
                    ),
                ),
            )
            .await?;
        return Ok(());
    }

    let (guild_id, channel_id) = {
        let guild_id = interaction.guild_id.unwrap();
        let user = interaction.user.id;

        if let Some(g) = ctx.cache.guild(guild_id) {
            // TODO: no voice
            let vs = g.voice_states.get(&user).unwrap();
            (guild_id, vs.channel_id.unwrap())
        } else {
            interaction
                .create_response(
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
                )
                .await?;

            // TODO: guild err
            error!("guild none");
            return Ok(());
            // (GuildId::new(0), ChannelId::new(0))
        }
    };

    let interact_resp = play_audio(ctx, guild_id, channel_id, final_url, filename)
        .await
        .unwrap();

    interaction.create_response(ctx, interact_resp).await?;
    Ok(())
}

async fn play_audio(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    url: Option<Url>,
    filename: String,
) -> Result<CreateInteractionResponse, ()> {
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

        let url = url.unwrap();
        let src = YoutubeDl::new(data.http.clone(), url.to_string());
        let song = handler.play_input(src.into());

        let mut title = String::new();

        if url.to_string().contains("cdn.discordapp.com") {
            title = filename;
        } else if url.to_string().contains("youtu") {
            let pairs = url.query_pairs();
            for pair in pairs {
                if pair.0 == "v" {
                    if let Ok(title1) = crate::youtube::get_video_title(&pair.1).await {
                        title = title1;
                    }
                }
            }
        } else {
            title = url.to_string();
        }

        // TODO: persist loop setting
        let _ = song.enable_loop();
        let _ = song.set_volume(0.5);
        data.track_handles.insert(guild_id, song);
        Ok(CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .color(Colour::new(COLOR_OK))
                    .title("Now Playing")
                    .description(format!("[{}]({})", title, url))
                    .timestamp(Timestamp::now()),
            ),
        ))
    } else {
        // TODO: error
        error!("Songbird get none");
        // check_msg(
        //     msg.channel_id
        //         .say(&ctx.http, "Not in a voice channel to play in")
        //         .await,
        // );
        Ok(CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().embed(
                CreateEmbed::new()
                    .color(Colour::new(COLOR_ERROR))
                    .title("Error")
                    .description("Could not get Songbird manager for guild")
                    .timestamp(Timestamp::now()),
            ),
        ))
    }
}

pub async fn run_component(
    ctx: &Context,
    interaction: &ComponentInteraction,
) -> Result<(), serenity::Error> {
    let final_url: Option<Url>;
    if let ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind {
        let id = values[0].clone();
        let url = format!("https://youtube.com/watch?v={}", id);
        let url = Url::parse(&url);
        final_url = Some(url.unwrap());
    } else {
        error!("component interaction not string??");
        return Ok(());
    }

    let (guild_id, channel_id) = {
        let guild_id = interaction.guild_id.unwrap();
        let user = interaction.user.id;

        if let Some(g) = ctx.cache.guild(guild_id) {
            // TODO: no voice
            let vs = g.voice_states.get(&user).unwrap();
            (guild_id, vs.channel_id.unwrap())
        } else {
            interaction
                .create_response(
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
                )
                .await?;

            // TODO: guild err
            error!("guild none");
            return Ok(());
            // (GuildId::new(0), ChannelId::new(0))
        }
    };
    let interact_resp = play_audio(ctx, guild_id, channel_id, final_url, String::new())
        .await
        .unwrap();

    interaction.create_response(ctx, interact_resp).await?;

    // if let ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind {

    // }
    Ok(())
}

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

// fn truncate_in_place(s: &mut String, max_chars: usize) {
//     let bytes = truncate(s, max_chars).len();
//     s.truncate(bytes);
// }

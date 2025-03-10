use log::{error, info, warn};
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::{TrackEvent, tracks::PlayMode};

use crate::{COLOR_ERROR, COLOR_OK, UserData};

pub fn register() -> CreateCommand {
    CreateCommand::new("pause").description("Pause/plays the active music")
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let guild_id = interaction.guild_id.unwrap();
    let mut typemap = ctx.data.write().await;
    let data = typemap.get_mut::<UserData>().unwrap();
    let track = data.track_handles.get_mut(&guild_id).unwrap();

    let info = track.get_info().await.unwrap();
    match info.playing {
        PlayMode::Pause => {
            let _ = track.play();
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(
                            CreateEmbed::new()
                                .color(Colour::new(COLOR_OK))
                                .description("Resuming track"),
                        ),
                    ),
                )
                .await?;
        }
        PlayMode::Play => {
            let _ = track.pause();
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(
                            CreateEmbed::new()
                                .color(Colour::new(COLOR_OK))
                                .title("Pausing track")
                                .timestamp(Timestamp::now()),
                        ),
                    ),
                )
                .await?;
        }
        _ => {
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().embed(
                            CreateEmbed::new()
                                .color(Colour::new(COLOR_ERROR))
                                .description("Could not pause or play track")
                                .title("Error")
                                .timestamp(Timestamp::now()),
                        ),
                    ),
                )
                .await?;
        }
    }

    Ok(())
}

use log::{error, info, warn};
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::TrackEvent;
use songbird::tracks::LoopState;

use crate::{COLOR_OK, UserData};

pub fn register() -> CreateCommand {
    CreateCommand::new("loop").description("Toggle looping (default on)")
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let guild_id = interaction.guild_id.unwrap();
    let d = ctx.data.clone();
    let mut typemap = d.write().await;
    let data = typemap.get_mut::<UserData>().unwrap();
    if let Some(track) = data.track_handles.get_mut(&guild_id) {
        let loops: LoopState = track.get_info().await.unwrap().loops;
        let is_looping = match loops {
            LoopState::Finite(0) => false,
            LoopState::Finite(_) | LoopState::Infinite => true,
        };

        if is_looping {
            let _ = track.disable_loop();
        } else {
            let _ = track.enable_loop();
        }
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .color(Colour::new(COLOR_OK))
                            .description(format!("Set looping to {}", !is_looping)),
                    ),
                ),
            )
            .await?;
    }

    Ok(())
}

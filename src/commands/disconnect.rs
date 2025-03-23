use log::{error, info, warn};
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::TrackEvent;

use crate::{COLOR_OK, UserData};

pub fn register() -> CreateCommand {
    CreateCommand::new("disconnect")
        .description("Disconnect the bot from the actively playing voice channel")
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let guild_id = interaction.guild_id.unwrap();
    let mut typemap = ctx.data.write().await;
    let data = typemap.get_mut::<UserData>().unwrap();
    let track = data.track_handles.get_mut(&guild_id);
    if let Some(track) = track {
        let _ = track.stop();
    }
    let _ = data.songbird.remove(guild_id).await;

    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(
                    CreateEmbed::new()
                        .color(Colour::new(COLOR_OK))
                        .title("Disconnected")
                        .timestamp(Timestamp::now()),
                ),
            ),
        )
        .await?;

    Ok(())
}

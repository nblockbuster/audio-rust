use log::{error, info, warn};
use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::TrackEvent;

use crate::{COLOR_ERROR, COLOR_OK, UserData};

pub fn register() -> CreateCommand {
    CreateCommand::new("volume")
        .description("Adjust volume of the bot for everyone")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Number,
                "number",
                "A number from 0 to 100, default 50",
            )
            .min_number_value(0.0)
            .max_number_value(100.0)
            .required(true),
        )
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    let guild_id = interaction.guild_id.unwrap();
    let d = ctx.data.clone();
    let mut typemap = d.write().await;
    let data = typemap.get_mut::<UserData>().unwrap();
    let handle = data.track_handles.get_mut(&guild_id).unwrap();

    if let Some(ResolvedOption {
        value: ResolvedValue::Number(num),
        ..
    }) = interaction.data.options().first().cloned()
    {
        let _ = handle.set_volume(num as f32 / 100.0);
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .color(Colour::new(COLOR_OK))
                            .description(format!("Set volume to {}", num)),
                    ),
                ),
            )
            .await?;
    } else {
        error!("invalid volume?");
        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .color(Colour::new(COLOR_ERROR))
                            .description("Could not set volume"),
                    ),
                ),
            )
            .await?;
    }

    Ok(())
}

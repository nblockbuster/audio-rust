use serenity::builder::*;
use serenity::model::prelude::*;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("search")
        .description("Search for a song")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "query",
                "Search for a song on YouTube",
            )
            .required(true),
        )
}

pub async fn run(ctx: &Context, interaction: &CommandInteraction) -> Result<(), serenity::Error> {
    if let Some(ResolvedOption {
        value: ResolvedValue::String(search_str),
        ..
    }) = interaction.data.options().first().cloned()
    {
        let resp = super::play::search(search_str).await;
        interaction.create_response(&ctx.http, resp).await?;
    }
    Ok(())
}

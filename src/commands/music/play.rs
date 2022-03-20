use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::*;
use serenity::prelude::*;
use tracing::info;

#[command]
pub async fn play_msg(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Added to queue.").await?;
    Ok(())
}

pub async fn _play_itn(ctx: &Context, command: &ApplicationCommandInteraction) {
    info!("Creating response...");
    let _res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content("Added to queue."))
        })
        .await;
    info!("Response created.");
}

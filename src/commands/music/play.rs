use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::model::prelude::Message;
use serenity::prelude::Context;
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

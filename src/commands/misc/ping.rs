use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::MessageFlags;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::model::prelude::Message;
use serenity::prelude::Context;
use tracing::{error, info};

#[command]
pub async fn ping_msg(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

#[allow(unused)]
pub async fn command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
) {
    info!("Creating response...");
    let _res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.flags(MessageFlags::EPHEMERAL);
                    message.content("Pongus!")
                })
        })
        .await;
    info!("Response created.");
}
#[allow(dead_code)]
pub async fn register(ctx: &Context) {
    if let Err(err) = Command::create_global_application_command(&*ctx.http, |command| {
        command.name("pingus").description("An amazing command")
    })
    .await
    {
        error!("Could not register pingus command! {}", err.to_string());
        panic!()
    }
}

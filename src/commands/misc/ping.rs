use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::interactions::application_command::{ApplicationCommand, ApplicationCommandInteraction};
use serenity::model::prelude::*;
use serenity::prelude::*;
use tracing::{error, info};

#[command]
pub async fn ping_msg(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

#[allow(unused)]
pub async fn command(ctx: &Context, command: &ApplicationCommandInteraction, mongo_client: &mongodb::Client) {
    info!("Creating response...");
    let _res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                    message.content("Pongus!")
                })
        })
        .await;
    info!("Response created.");
}
#[allow(dead_code)]
pub async fn register(ctx: &Context) {
    if let Err(err) = ApplicationCommand::create_global_application_command(&*ctx.http, |command| {
        command.name("pingus").description("An amazing command")
    })
        .await {
        error!("Could not register pingus command! {}", err.to_string());
        panic!()
    }
}

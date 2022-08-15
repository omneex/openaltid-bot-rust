use serenity::model::prelude::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::interaction::MessageFlags;
use serenity::utils::Colour;
use serenity::{
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
    prelude::Context,
};
use tracing::{error, warn};

pub async fn interaction_error(
    err_message: &str,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    warn!("Interaction Error: {}", err_message);

    let res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.flags(MessageFlags::EPHEMERAL);
                    message.embed(|embed| {
                        embed
                            .title("Uh Oh!")
                            .description("Something went wrong during that.")
                            .field("Reason", err_message, false)
                            .color(Colour::from_rgb(255, 0, 0))
                    })
                })
        })
        .await;

    if let Err(err) = res {
        error!(
            "An error occurred while sending an error interaction reply. {}",
            err
        );
    }
}

pub async fn interaction_error_comp(
    err_message: &str,
    command: &MessageComponentInteraction,
    ctx: &Context,
) {
    warn!("Interaction Error: {}", err_message);

    let res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.flags(MessageFlags::EPHEMERAL);
                    message.embed(|embed| {
                        embed
                            .title("Uh Oh!")
                            .description("Something went wrong during that.")
                            .field("Reason", err_message, false)
                            .color(Colour::from_rgb(255, 0, 0))
                    })
                })
        })
        .await;

    if let Err(err) = res {
        error!(
            "An error occurred while sending an error interaction reply. {}",
            err
        );
    }
}

pub async fn channel_message_error(
    err_message: &str,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    warn!("Sending error message: {}", err_message);
    let res = command.channel_id.send_message(&ctx.http, |message| {
        message.embed(|embed| {
            embed
                .title("Yikes!")
                .description("An error occurred and the bot was not able to reply with an interaction! This is a channel message fallback.")
                .field("Reason", err_message, false)
                .color(Colour::from_rgb(255, 0, 0))
        })
    }).await;
    if let Err(err) = res {
        error!(
            "An error occurred while sending an error interaction reply. {}",
            err
        );
    }
}

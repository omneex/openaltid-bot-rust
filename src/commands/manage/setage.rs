use super::super::super::dbmodels::guild::Guild as GuildStruct;
use super::super::common::interaction_error::{channel_message_error, interaction_error};
use crate::commands::common::permissions_check::check_if_mod;
use crate::commands::common::slash_commands::{extract_vec, get_int};
use crate::log::{debug, warn};
use mongodb::bson::{doc, Bson};
use mongodb::*;
use serenity::model::prelude::application_command::*;
use serenity::model::prelude::*;
use serenity::prelude::*;
use tracing::{error, info, instrument};

#[instrument(skip(ctx, mongo_client))]
pub async fn command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
) {
    // Check if the user is a mod.
    match check_if_mod(&ctx, &command, &mongo_client).await {
        Ok(is_mod) => {
            if !is_mod {
                return;
            }
            {
                interaction_error("You must be a mod to use this command.", command, ctx).await;
            }
        }
        Err(err) => {
            warn!("{}", err);
            interaction_error(err, command, ctx).await;
            return;
        }
    }
    let command_options = command.data.options.clone();
    let mut num_days_bson: Bson = bson::Bson::Boolean(false);
    for tup in extract_vec(&command_options).await {
        match tup.0 {
            "num_of_days" => {
                // Extract an int from the options and convert it into bson.
                if let Some(x) = get_int(tup.1).await {
                    let num_days = x;
                    num_days_bson = match bson::to_bson(&num_days) {
                        Ok(bson_data) => bson_data,
                        Err(err) => {
                            error!("{:?}", err);
                            interaction_error("Could not convert input properly.", &command, &ctx)
                                .await;
                            return;
                        }
                    }
                } else {
                    interaction_error(
                        "'num_days' param was invalid, make sure you gave an integer (no decimal).",
                        command,
                        ctx,
                    )
                    .await;
                    return;
                }
            }
            _ => {
                warn!("Unrecognized parameter given.")
            }
        }
    }

    // Extract the Guild ID as a string.
    let guild_id_str = match command.guild_id {
        None => {
            interaction_error("This command must be run in a guild.", &command, &ctx).await;
            return;
        }
        Some(x) => x.0.to_string(),
    };

    let collection: Collection<GuildStruct> = mongo_client.database("bot").collection("guilds");
    let _ = match collection
        .update_one(
            doc! {"guild_ID": guild_id_str},
            doc! {"$set": {"verification_age": &num_days_bson}},
            None,
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            error!("{:?}", err);
            interaction_error("Could not update the database.", &command, &ctx).await;
            return;
        }
    };

    let res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                    message.content(format!(
                        "The minimum age to bypass verification is now set to: {} days",
                        num_days_bson
                    ))
                })
        })
        .await;

    if let Err(err) = res {
        error!("{:?}", err);
        channel_message_error("Could not send interaction message.", command, ctx).await;
    }
}

#[instrument(skip(ctx))]
pub async fn register(ctx: &Context) {
    let result = ApplicationCommand::create_global_application_command(&*ctx.http, |command| {
        command
            .name("setminage")
            .description("Set the minimum age to avoid verification")
            .create_option(|opt| {
                opt.name("age")
                    .description("The channel to send logs to.")
                    .kind(ApplicationCommandOptionType::Integer)
                    .required(true)
            })
    })
    .await;

    match result {
        Ok(command) => {
            info!("Command {:?} registered successfully.", command);
            command
        }
        Err(error) => {
            error!("Could not create guild command! {:?}", error);
            return;
        }
    };
}

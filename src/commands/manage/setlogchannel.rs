use super::super::super::dbmodels::guild::Guild as GuildStruct;
use crate::commands::common::interaction_error::interaction_error;
use crate::commands::common::permissions_check::check_if_mod;
use mongodb::bson::doc;
use mongodb::{bson, Client, Collection};
use serenity::client::Context;
use serenity::model::interactions::application_command::{
    ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOptionType,
};
use serenity::model::interactions::{
    InteractionApplicationCommandCallbackDataFlags, InteractionResponseType,
};
use tracing::*;

pub async fn command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    mongo_client: &Client,
) {
    // Check if mod already.
    match check_if_mod(ctx, command, mongo_client).await {
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

    let options = command.data.options.clone();
    let mut channel_id_string: String = "".to_string();
    for tup in super::super::common::slash_commands::extract_vec(&options).await {
        if tup.0 == "channel" {
            if let Some(x) = super::super::common::slash_commands::get_channel(tup.1).await {
                channel_id_string = x.id.0.to_string();
            } else {
                interaction_error("'channel' param was invalid.", command, ctx).await;
                return;
            }
        }
    }

    let guild_id_str = match command.guild_id {
        None => {
            interaction_error("This command must be run in a guild.", command, ctx).await;
            return;
        }
        Some(x) => x.0.to_string(),
    };

    let channel_bson = match bson::to_bson(&channel_id_string) {
        Ok(bson) => bson,
        Err(err) => {
            error!("{}", err);
            interaction_error("Could not convert role ID to bson.", command, ctx).await;
            return;
        }
    };

    let collection: Collection<GuildStruct> = mongo_client.database("bot").collection("guilds");
    let update_res = match collection
        .update_one(
            doc! {"guild_ID": guild_id_str},
            doc! {"$set": {"verification_logs_channel_ID": &channel_bson}},
            None,
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            error!("{:?}", err);
            interaction_error("Could not update the database.", command, ctx).await;
            return;
        }
    };
    debug!("{:?}", update_res);

    info!("Creating response...");
    let res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL);
                    message.content(format!(
                        "Set the logs channel to: <#{}> ID: {}",
                        &channel_id_string, &channel_id_string
                    ))
                })
        })
        .await;
    if let Err(err) = res {
        error!("{}", err);
        super::super::common::interaction_error::channel_message_error(
            "Could not send interaction message.",
            command,
            ctx,
        )
        .await;
    } else {
        info!("Response created.");
    }
}

#[instrument(skip(ctx))]
pub async fn register(ctx: &Context) {
    let result = ApplicationCommand::create_global_application_command(&*ctx.http, |command| {
        command
            .name("setlogchannel")
            .description(
                "Set which channel to send logs to, omit channel to disable. Mod only command.",
            )
            .create_option(|opt| {
                opt.name("channel")
                    .description("The channel to send logs to.")
                    .kind(ApplicationCommandOptionType::Channel)
                    .required(true)
            })
    })
    .await;

    match result {
        Ok(command) => {
            info!("Command {:?} registered successfully.", command);
        }
        Err(error) => {
            error!("Could not create guild command! {:?}", error);
        }
    };
}

use mongodb::bson;
use mongodb::bson::doc;
use mongodb::Collection;
use serenity::model::application::command::Command;
use serenity::model::application::command::CommandOptionType;
use serenity::model::application::interaction::MessageFlags;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::model::prelude::Role;
use serenity::prelude::Context;
use tracing::debug;
use tracing::{error, info, instrument, warn};

use crate::commands::common::interaction_error::{channel_message_error, interaction_error};
use crate::commands::common::permissions_check::check_if_mod;
use crate::commands::common::slash_commands::extract_vec;
use crate::dbmodels::guild::Guild as GuildStruct;

#[instrument(skip(ctx, mongo_client))]
pub async fn command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
) {
    // Check if mod already.
    match check_if_mod(ctx, command, mongo_client).await {
        Ok(is_mod) => {
            if !is_mod {
                interaction_error("You must be a mod to use this command.", command, ctx).await;
                return;
            }
        }
        Err(err) => {
            warn!("{}", err);
            interaction_error(err, command, ctx).await;
            return;
        }
    }

    let options = command.data.options.clone();
    let mut role_opt: Option<Role> = None;
    for tup in extract_vec(&options).await {
        if tup.0 == "role" {
            if let Some(x) = super::super::common::slash_commands::get_role(tup.1).await {
                role_opt = Some(x)
            } else {
                interaction_error("'role' param was invalid.", command, ctx).await;
                return;
            }
        }
    }

    // Check to make sure its there!
    let role = match role_opt {
        None => {
            interaction_error("No role provided.", command, ctx).await;
            return;
        }
        Some(role) => role.id.0.to_string(),
    };

    let guild_id_str = match command.guild_id {
        None => {
            interaction_error("This command must be run in a guild.", command, ctx).await;
            return;
        }
        Some(id) => id.0.to_string(),
    };

    let role_bson = match bson::to_bson(&role) {
        Ok(bson) => bson,
        Err(err) => {
            error!("{}", err);
            interaction_error("Could not convert role ID to bson.", command, ctx).await;
            return;
        }
    };

    let collection: Collection<GuildStruct> = mongo_client.database("botdb").collection("guilds");
    let update_res = match collection
        .update_one(
            doc! {"guild_ID": guild_id_str},
            doc! {"$set": {"mod_role_ID": &role_bson}},
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
    debug!("Creating response...");
    let res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.flags(MessageFlags::EPHEMERAL);
                    message.content(format!(
                        "The mod role is now set to <@&{}> ID: {}",
                        &role, &role
                    ))
                })
        })
        .await;
    if let Err(err) = res {
        error!("{}", err);
        channel_message_error("Could not send interaction message.", command, ctx).await;
    } else {
        info!("Response created.");
    }
}

#[instrument(skip(ctx))]
pub async fn register(ctx: &Context) {
    let result = Command::create_global_application_command(&*ctx.http, |command| {
        command
            .name("setmodrole")
            .description("Add an additional role to be able to act as mod. Mod only command.")
            .create_option(|opt| {
                opt.name("role")
                    .description("The role you want to set.")
                    .kind(CommandOptionType::Role)
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

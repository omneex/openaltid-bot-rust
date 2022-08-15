use mongodb::Collection;
use mongodb::bson;
use mongodb::bson::doc;
use serenity::model::application::command::CommandOptionType;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::MessageFlags;
use serenity::model::prelude::Role;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;
use tracing::{error, info, warn, instrument};

use crate::commands::common::interaction_error::{interaction_error};
use crate::commands::common::permissions_check::check_if_mod;
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
    for tup in super::super::common::slash_commands::extract_vec(&options).await {
        if tup.0 == "role" {
            if let Some(x) = super::super::common::slash_commands::get_role(tup.1).await {
                role_opt = Some(x)
            } else {
                super::super::common::interaction_error::interaction_error(
                    "'role' param was invalid.",
                    command,
                    ctx,
                )
                .await;
                return;
            }
        }
    }

    let role = match role_opt {
        None => {
            super::super::common::interaction_error::interaction_error(
                "No role provided.",
                command,
                ctx,
            )
            .await;
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

    // TODO set the role in the database
    let role_bson = match bson::to_bson(&role) {
        Ok(bson) => bson,
        Err(err) => {
            error!("{}", err);
            interaction_error("Could not convert role ID to bson.", command, ctx).await;
            return;
        }
    };

    let collection: Collection<GuildStruct> = mongo_client.database("botdb").collection("guilds");
    let _ = match collection
        .update_one(
            doc! {"guild_ID": guild_id_str},
            doc! {"$set": {"verification_role_ID": &role_bson}},
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

    info!("Creating response...");
    let res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.flags(MessageFlags::EPHEMERAL);
                    message.content(format!("Set the verification role to: <@&{}>", role))
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
    let result = Command::create_global_application_command(&*ctx.http, |command| {
        command
            .name("setverifiedrole")
            .description(
                "Set the verification role, this role is given when verified. Mod only command.",
            )
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

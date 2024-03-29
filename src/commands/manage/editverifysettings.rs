use mongodb::bson;
use mongodb::bson::doc;
use mongodb::Collection;
use serenity::model::application::command::Command;
use serenity::model::application::command::CommandOptionType;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;
use tracing::{error, info, instrument, warn};

use crate::commands::common::interaction_error::{channel_message_error, interaction_error};
use crate::commands::common::permissions_check::check_if_mod;
use crate::dbmodels::guild::Guild as GuildStruct;

#[instrument(skip(ctx, mongo_client))]
pub async fn command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
) {
    // Check if the user is a mod.
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

    let command_options = command.data.options.clone();
    // Extract the Guild ID as a string.
    let guild_id_str = match command.guild_id {
        None => {
            interaction_error("This command must be run in a guild.", command, ctx).await;
            return;
        }
        Some(x) => x.0.to_string(),
    };

    let mut values_to_update = doc! {};
    let accept_args = [
        "zero_point",
        "difficulty_addition",
        "mfa_bonus",
        "premium_bonus",
        "preferred_num_of_accounts",
    ];
    for tup in super::super::common::slash_commands::extract_vec(&command_options).await {
        if accept_args.contains(&tup.0) {
            if let Some(x) = super::super::common::slash_commands::get_int(tup.1).await {
                let bson_val = match bson::to_bson(&x) {
                    Ok(bson_val) => bson_val,
                    Err(err) => {
                        error!("Could not convert val ({:?}) to bson. {:?}", x, err);
                        interaction_error("Could not convert inputs.", command, ctx).await;
                        return;
                    }
                };
                values_to_update.insert(format!("guild_settings.{}", &tup.0), bson_val);
            } else {
                interaction_error("param was invalid.", command, ctx).await;
                return;
            }
        }
    }

    let update_statement = doc! {"$set": values_to_update};

    let collection: Collection<GuildStruct> = mongo_client.database("botdb").collection("guilds");
    let _ = match collection
        .update_one(doc! {"guild_ID": &guild_id_str}, update_statement, None)
        .await
    {
        Ok(res) => res,
        Err(err) => {
            error!("{:?}", err);
            interaction_error("Could not update the database.", command, ctx).await;
            return;
        }
    };

    let collection: Collection<GuildStruct> = mongo_client.database("botdb").collection("guilds");
    let settings_doc = match collection
        .find_one(doc! {"guild_ID": guild_id_str}, None)
        .await
    {
        Ok(res) => match res {
            None => {
                interaction_error(
                    "Could not find guild in the database. This really shouldn't ever happen.",
                    command,
                    ctx,
                )
                .await;
                return;
            }
            Some(doc) => doc,
        },
        Err(err) => {
            error!("{:?}", err);
            interaction_error("Could not get guild from the database.", command, ctx).await;
            return;
        }
    };

    let res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.embed(|embed| {
                        embed
                            .title("New Server Settings")
                            .field("Mod Role:", format!("**{}**\n<@&{}>", settings_doc.mod_role_ID, settings_doc.mod_role_ID), true)
                            .field("Verification Age:", format!("\n**{}** days", settings_doc.verification_age), true)
                            .field("Logs Channel:", format!("**{}**\n<#{}>", settings_doc.verification_logs_channel_ID, settings_doc.verification_logs_channel_ID), true)
                            .field("Verification Settings:", format!(
                                "Calculated Minimum: {:.0}\nDifficulty Addition:   {}\nMFA Bonus:    {}\nPreferred Num of Accounts: {}\nPremium Bonus:  {}\nZero Point:   {}",
                                (settings_doc.guild_settings.zero_point * (settings_doc.guild_settings.preferred_num_of_accounts as i64) + settings_doc.guild_settings.difficulty_addition) as u64,
                                settings_doc.guild_settings.difficulty_addition,
                                settings_doc.guild_settings.mfa_bonus,
                                settings_doc.guild_settings.preferred_num_of_accounts,
                                settings_doc.guild_settings.premium_bonus,
                                settings_doc.guild_settings.zero_point),
                            false)
                            .footer(|footer| footer.text("Powered by Open/Alt.ID"))
                    })
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
    let result = Command::create_global_application_command(&*ctx.http, |command| {
        command
            .name("editverifysettings")
            .description("Set the settings for the verification algorithm.")
            .create_option(|opt| {
                opt.name("zero_point")
                    .description("Base point value required per account.")
                    .kind(CommandOptionType::Integer)
                    .required(false)
            })
            .create_option(|opt| {
                opt.name("mfa_bonus")
                    .description("Bonus value if user has multi factor auth on.")
                    .kind(CommandOptionType::Integer)
                    .required(false)
            })
            .create_option(|opt| {
                opt.name("premium_bonus")
                    .description("Bonus value if user has premium.")
                    .kind(CommandOptionType::Integer)
                    .required(false)
            })
            .create_option(|opt| {
                opt.name("preferred_num_of_accounts")
                    .description(
                        "Number of accounts preferred. Acts as a multiplier to zero_point.",
                    )
                    .kind(CommandOptionType::Integer)
                    .required(false)
            })
            .create_option(|opt| {
                opt.name("difficulty_addition")
                    .description("Arbitrary value to add to required amount.")
                    .kind(CommandOptionType::Integer)
                    .required(false)
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

use super::super::super::dbmodels::guild::Guild as GuildStruct;
use super::super::common::interaction_error::{channel_message_error, interaction_error};
use crate::commands::common::permissions_check::check_if_mod;
use crate::commands::common::slash_commands::{extract_vec, get_int};
use crate::log::warn;
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

    // Extract the Guild ID as a string.
    let guild_id_str = match command.guild_id {
        None => {
            interaction_error("This command must be run in a guild.", command, ctx).await;
            return;
        }
        Some(x) => x.0.to_string(),
    };

    let collection: Collection<GuildStruct> = mongo_client.database("botdb").collection("guilds");
    let settings_doc = match collection
        .find_one(doc! {"guild_ID": guild_id_str}, None)
        .await
    {
        Ok(res) => match res {
            None => {
                interaction_error("Could not find guild in the database. This really shouldn't ever happen.", command, ctx).await;
                return
            }
            Some(doc) => doc
        },
        Err(err) => {
            error!("{:?}", err);
            interaction_error("Could not get guild from the database.", command, ctx).await;
            return
        }
    };

    let res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.create_embed(|embed| {
                        embed
                            .title("Current Server Settings")
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
    let result = ApplicationCommand::create_global_application_command(&*ctx.http, |command| {
        command
            .name("currentsettings")
            .description("Gets the current settings of the server.")

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

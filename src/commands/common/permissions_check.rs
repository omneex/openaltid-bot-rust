use crate::dbmodels::guild::Guild;
use mongodb::bson::doc;
use mongodb::Client;
use serenity::{
    client::Context,
    model::{
        application::interaction::application_command::ApplicationCommandInteraction,
        prelude::interaction::message_component::MessageComponentInteraction,
    },
};
use tracing::{debug, error};

pub async fn check_if_mod(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    mongo_client: &Client,
) -> Result<bool, &'static str> {
    // Check if the user is an admin, admins always have permission.
    match &command.member {
        None => {}
        Some(mem) => match mem.permissions {
            None => {}
            Some(perms) => {
                if perms.administrator() {
                    debug!("User had admin perms - Allowing");
                    return Ok(true);
                }
            }
        },
    }

    // Get the u64 of the Guild ID
    let guild_id = match command.guild_id {
        None => return Err("You must run this command in a guild."),
        Some(id) => id.0,
    };

    // Try to get the guild from the database, returns an option if the guild was found.
    let guild_doc_opt = match mongo_client
        .database("botdb")
        .collection("guilds")
        .find_one(doc! {"guild_ID": guild_id.to_string()}, None)
        .await
    {
        Ok(col_opt) => col_opt,
        Err(err) => {
            error!("{:?}", err);
            return Err("Could not retrieve guild from database.");
        }
    };

    // Try to extract the guild doc from the option.
    let guild_doc: Guild = match guild_doc_opt {
        None => return Err("Could not find guild in database."),
        Some(doc) => doc,
    };

    debug!("Permission check in {:?}", command.guild_id);

    // Parse the mod role ID from the database doc.
    let role_id_u64: u64 = match guild_doc.mod_role_ID.parse::<u64>() {
        Ok(num) => num,
        Err(_) => return Err("Could not parse mod role ID."),
    };

    debug!("Role ID as a u64 {:?}", role_id_u64);

    // Check if the user has the mod role, and return the Ok(bool)
    let allowed: bool = match command
        .user
        .has_role(&ctx.http, guild_id, role_id_u64)
        .await
    {
        Ok(allowed) => allowed,
        Err(_) => return Err("Failed to check if user has mod role."),
    };
    Ok(allowed)
}

pub async fn check_if_mod_comp(
    ctx: &Context,
    command: &MessageComponentInteraction,
    mongo_client: &Client,
) -> Result<bool, &'static str> {
    // Check if the user is an admin, admins always have permission.
    match &command.member {
        None => {}
        Some(mem) => match mem.permissions {
            None => {}
            Some(perms) => {
                if perms.administrator() {
                    debug!("User had admin perms - Allowing");
                    return Ok(true);
                }
            }
        },
    }

    // Get the u64 of the Guild ID
    let guild_id = match command.guild_id {
        None => return Err("You must run this command in a guild."),
        Some(id) => id.0,
    };

    // Try to get the guild from the database, returns an option if the guild was found.
    let guild_doc_opt = match mongo_client
        .database("botdb")
        .collection("guilds")
        .find_one(doc! {"guild_ID": guild_id.to_string()}, None)
        .await
    {
        Ok(col_opt) => col_opt,
        Err(err) => {
            error!("{:?}", err);
            return Err("Could not retrieve guild from database.");
        }
    };

    // Try to extract the guild doc from the option.
    let guild_doc: Guild = match guild_doc_opt {
        None => return Err("Could not find guild in database."),
        Some(doc) => doc,
    };

    debug!("Permission check in {:?}", command.guild_id);

    // Parse the mod role ID from the database doc.
    let role_id_u64: u64 = match guild_doc.mod_role_ID.parse::<u64>() {
        Ok(num) => num,
        Err(_) => return Err("Could not parse mod role ID."),
    };

    debug!("Role ID as a u64 {:?}", role_id_u64);

    // Check if the user has the mod role, and return the Ok(bool)
    let allowed: bool = match command
        .user
        .has_role(&ctx.http, guild_id, role_id_u64)
        .await
    {
        Ok(allowed) => allowed,
        Err(_) => return Err("Failed to check if user has mod role."),
    };
    Ok(allowed)
}

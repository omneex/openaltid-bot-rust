use crate::commands::manage::*;
use crate::commands::misc::ping::command as pingcommand;
use crate::commands::verification::*;
use mongodb::Client;
use redis::AsyncCommands;
use redis::RedisError;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::command::Command;
use serenity::model::prelude::command::CommandPermissionType;
use serenity::model::prelude::interaction::Interaction;
use serenity::model::prelude::CommandId;
use serenity::model::prelude::CommandPermissionId;
use serenity::model::prelude::GuildId;
use serenity::model::prelude::RoleId;
use serenity::prelude::Context;
use tracing::*;

pub async fn register(ctx: &Context) {
    // Do all command registrations here.
    // If a command fails to register it will panic.
    info!("Registering commands...");
    setage::register(ctx).await;
    setlogchannel::register(ctx).await;
    setmodrole::register(ctx).await;
    setverificaitonrole::register(ctx).await;
    add_connection::register(ctx).await;
    remove_connection::register(ctx).await;
    verify::register(ctx).await;
    currentsettings::register(ctx).await;
    editverifysettings::register(ctx).await;
    info!("Done.");

    // Print out the currently registered commands.
    if let Err(err) = Command::get_global_application_commands(&*ctx.http)
        .await
        .map(|commands| {
            commands.iter().for_each(|command| {
                info!(
                    "Application command {} with ID {} is registered.",
                    command.name, command.id
                );
            })
        })
    {
        error!("Could not retrieve commands. {}", err.to_string())
    }
}

pub async fn handle_interactions(
    ctx: &Context,
    intn: Interaction,
    mongo_client: &mongodb::Client,
    redis_client: &redis::Client,
) {
    match intn {
        Interaction::Ping(_) => {}
        Interaction::ApplicationCommand(a_command) => {
            handle_commands(&ctx, &a_command, mongo_client, redis_client).await;
        }
        Interaction::MessageComponent(m_component) => {
            handle_components(&ctx, &m_component, mongo_client).await;
        }
        _ => {}
    }
}

async fn handle_commands(
    ctx: &&Context,
    a_command: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
    redis_client: &redis::Client,
) {
    info!(
        "Application command '{}'({}) invoked by user '{}'({}) in Ch.{} Gld.{}",
        a_command.data.name,
        a_command.id.0,
        a_command.user.name,
        a_command.user.id,
        a_command.channel_id.0,
        a_command.guild_id.unwrap_or(GuildId(0))
    );

    match a_command.data.name.as_str() {
        "pingus" => {
            pingcommand(ctx, a_command, mongo_client).await;
        }
        "verify" => {
            let mut conn = match redis_client.get_multiplexed_tokio_connection().await {
                Ok(conn) => conn,
                Err(err) => {
                    panic!("REDIS ERROR - FAILED TO GET CONNECTION - {:?}", err);
                }
            };
            let val: Result<String, RedisError> = conn.get("hello").await;
            debug!("{:?}", val);
            verify::command(ctx, a_command, mongo_client, &mut conn).await;
        }
        "addconnection" => {
            add_connection::command(ctx, a_command, mongo_client).await;
        }
        "removeconnection" => {
            remove_connection::command(ctx, a_command, mongo_client).await;
        }
        "setminage" => {
            setage::command(ctx, a_command, mongo_client).await;
        }
        "setlogchannel" => {
            setlogchannel::command(ctx, a_command, mongo_client).await;
        }
        "setmodrole" => {
            setmodrole::command(ctx, a_command, mongo_client).await;
        }
        "setverifiedrole" => {
            setverificaitonrole::command(ctx, a_command, mongo_client).await;
        }
        "currentsettings" => {
            currentsettings::command(ctx, a_command, mongo_client).await;
        }
        "editverifysettings" => {
            editverifysettings::command(ctx, a_command, mongo_client).await;
        }
        _ => {
            warn!("Command not found.");
        }
    };
}

async fn handle_components(
    ctx: &&Context,
    m_component: &MessageComponentInteraction,
    mongo_client: &Client,
) {
    let ids_split: Vec<&str> = m_component.data.custom_id.split(':').collect();
    let comp_type: &str = match ids_split.first() {
        Some(str_type) => *str_type,
        None => "none",
    };
    // TODO possibly avoid another split here by using this split again, but for now I dont want to edit the signiture
    match comp_type {
        "HelpButton" => verify::help_callback(ctx, m_component, mongo_client).await,
        "UndoAddConnection" => add_connection::undo_callback(ctx, m_component, mongo_client).await,
        "UndoRemoveConnection" => {
            remove_connection::undo_callback(ctx, m_component, mongo_client).await
        }
        _ => {
            warn!("Interaction not found.");
        }
    }
}

// pub async fn clear(ctx: &Context) {
//     info!("Clearing slash commands...");
//     let mut commands_to_del: Vec<(CommandId, String)> = vec![];
//     let _res = Command::get_global_application_commands(&*ctx.http)
//         .await
//         .map(|comms| {
//             comms.iter().for_each(|comm| {
//                 let name = comm.name.clone();
//                 let id = comm.id.clone();
//                 commands_to_del.push((id, name))
//             })
//         })
//     info!(
//         "There are {} command/s to be cleared.",
//         commands_to_del.len()
//     );
//     for x in 0..commands_to_del.len() {
//         info!(
//             "Deleting command '{}' with ID {}",
//             commands_to_del[x].1, commands_to_del[x].0
//         );
//         let _res =
//             Command::delete_global_application_command(&*ctx.http, commands_to_del[x].0)
//                 .await;
//     }
//     for guild in ctx.cache.guilds().await {
//         let commands = match ctx.http.get_guild_application_commands(guild.0).await {
//             Ok(commands) => commands,
//             Err(e) => panic!("{}",e)
//         };
//         for command in commands {
//             match ctx.http.delete_guild_application_command(guild.0, command.id.0).await {
//                 Ok(_) => {}
//                 Err(e) => panic!("{}", e)
//             }
//         };
//     }
//     info!("Commands cleared. Will now re-add commands.");
// }

#[instrument(skip(ctx, command))]
pub async fn add_admins_to_perms(
    ctx: &Context,
    command: Command,
    guild_id: GuildId,
) -> serenity::static_assertions::_core::result::Result<(), &'static str> {
    let mut admin_role_ids: Vec<RoleId> = vec![];
    // Get roles with admin
    match guild_id.roles(&*ctx.http).await {
        Ok(role_map) => {
            for role_tup in role_map {
                let (role_id, role) = role_tup;
                if role.permissions.administrator() && role.tags.bot_id == None {
                    admin_role_ids.push(role_id);
                }
            }
        }
        Err(_) => {
            error!("Could not retrieve the guild roles.");
            return Err("Could not retrieve the guild roles.");
        }
    };
    for id in &admin_role_ids {
        match guild_id
            .create_application_command_permission(&*ctx.http, command.id, |perms| {
                perms.create_permission(|perm_data| {
                    perm_data
                        .id(id.0)
                        .permission(true)
                        .kind(CommandPermissionType::Role)
                })
            })
            .await
        {
            Ok(_) => {}
            Err(_) => {
                error!("Failed to create perm.");
                return Err("Failed to create perm.");
            }
        }
    }

    let perm = guild_id
        .get_application_command_permissions(&ctx.http, command.id)
        .await;
    match perm {
        Ok(_) => {}
        Err(_) => {
            return Err("Failed to get permissions.");
        }
    }
    Ok(())
}

#[instrument(skip(ctx))]
pub async fn get_vec_of_perms(
    ctx: &Context,
    command_id: &CommandId,
    guild_id: &GuildId,
) -> serenity::static_assertions::_core::result::Result<
    Vec<(CommandPermissionId, bool)>,
    &'static str,
> {
    let mut vec_of_roles: Vec<(CommandPermissionId, bool)> = vec![];
    let perm = guild_id
        .get_application_command_permissions(&ctx.http, *command_id)
        .await;
    match perm {
        Ok(p) => {
            for pe in p.permissions {
                vec_of_roles.push((pe.id, pe.permission));
            }
        }
        Err(_) => {
            return Err("Failed to get permissions.");
        }
    }
    Ok(vec_of_roles)
}

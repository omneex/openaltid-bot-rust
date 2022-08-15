use mongodb::bson::doc;
use mongodb::bson::Document;
use mongodb::Collection;
use serenity::model::application::command::Command;
use serenity::model::application::command::CommandOptionType;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::component::ButtonStyle;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::model::user::User;
use serenity::prelude::Context;
use tracing::debug;
use tracing::{error, info, instrument, warn};

use crate::commands::common::interaction_error::interaction_error;
use crate::commands::common::interaction_error::interaction_error_comp;
use crate::commands::common::permissions_check::check_if_mod;
use crate::commands::common::permissions_check::check_if_mod_comp;
use crate::commands::common::slash_commands;

#[instrument(skip(ctx, mongo_client))]
pub async fn command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
) {
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

    let mut user: User = User::default();
    let mut account_type: String = "".to_string();
    let mut account_id: String = "".to_string();
    let mut params_received = 0;

    for tup in slash_commands::extract_vec(&options).await {
        match tup.0 {
            "user" => {
                if let Some(x) = slash_commands::get_user(tup.1).await {
                    params_received += 1;
                    user = x
                } else {
                    interaction_error("'user' param is invalid.", command, ctx).await;
                    return;
                }
            }
            "account_type" => {
                if let Some(x) = slash_commands::get_string(tup.1).await {
                    params_received += 1;
                    account_type = x
                } else {
                    interaction_error("'account_type' param is invalid.", command, ctx).await;
                    return;
                }
            }
            "account_id" => {
                if let Some(x) = slash_commands::get_string(tup.1).await {
                    params_received += 1;
                    account_id = x
                } else {
                    interaction_error("'account_id' param is invalid.", command, ctx).await;
                    return;
                }
            }
            _ => {}
        }
    }

    if params_received != 3 {
        interaction_error("Incorrect number of parameters.", command, ctx).await;
        return;
    }

    info!(
        "User {}, Account {}, ID {}",
        user.name, account_type, account_id
    );

    let user_id: i64 = user.id.0 as i64;
    let collection: Collection<Document> = mongo_client
        .database("verification_data")
        .collection("socialmediaaccounts ");
    let insert_res = collection
        .insert_one(
            doc! {
                "user_ID": &user_id.to_string(),
                "account_type": &account_type,
                "account_id": &account_id,
            },
            None,
        )
        .await;

    if insert_res.is_err() {
        super::super::common::interaction_error::interaction_error(
            "Could not insert account into database.",
            command,
            ctx,
        )
        .await;
        return;
    }

    info!("Creating response...");
    let res = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message
                        .embed(|embed| {
                            embed
                                .title("Connection Added to database!")
                                .description("You can undo this by clicking the button below.")
                                .field("User:", format!("<@{}>\n{}", &user.id, &user.id), false)
                                .field("Account:", &account_type, false)
                                .field("Account ID:", &account_id, false)
                        })
                        .components(|components| {
                            components.create_action_row(|row| {
                                row.create_button(|button| {
                                    button.style(ButtonStyle::Danger).label("UNDO").custom_id(
                                        format!(
                                            "UndoAddConnection:{}:{}:{}",
                                            &user_id, &account_type, &account_id
                                        ),
                                    )
                                })
                            })
                        })
                })
        })
        .await;

    match res {
        Ok(_) => {}
        Err(err) => {
            error!("{}", err)
        }
    }
    info!("Response created.");
}

pub async fn undo_callback(
    ctx: &Context,
    interaction: &MessageComponentInteraction,
    mongo_client: &mongodb::Client,
) {
    match check_if_mod_comp(ctx, interaction, mongo_client).await {
        Ok(is_mod) => {
            if !is_mod {
                interaction_error_comp("You must be a mod to use this command.", interaction, ctx)
                    .await;
                return;
            }
        }
        Err(err) => {
            warn!("{}", err);
            interaction_error_comp(err, interaction, ctx).await;
            return;
        }
    }

    let ids_split: Vec<&str> = interaction.data.custom_id.split(':').collect();
    debug!("{:?}", ids_split);

    let user_id = match ids_split.get(1) {
        Some(val) => *val,
        None => {
            error!("Invalid interaction data in UNDO callback.");
            "error"
        }
    };
    let account_type = match ids_split.get(2) {
        Some(val) => *val,
        None => {
            error!("Invalid interaction data in UNDO callback.");
            "error"
        }
    };
    let account_id = match ids_split.get(3) {
        Some(val) => *val,
        None => {
            error!("Invalid interaction data in UNDO callback.");
            "error"
        }
    };

    debug!("{} - {} - {}", user_id, account_type, account_id);

    let collection: Collection<Document> = mongo_client
        .database("verification_data")
        .collection("socialmediaaccounts ");
    let delete_res = collection
        .delete_one(
            doc! {
                "user_ID": user_id,
                "account_type": account_type,
                "account_id": account_id,
            },
            None,
        )
        .await;

    match delete_res {
        Ok(doc) => {
            if doc.deleted_count < 1 {
                debug!("delete_res - {:?}", doc);
                let _res = interaction
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message.embed(|embed| {
                                    embed
                                        .title("No changes made")
                                        .description("No database entries matched the given data.")
                                        .footer(|footer| footer.text("Powered by Open/Alt.ID"))
                                })
                            })
                    })
                    .await;
                return;
            }
        }
        Err(err) => {
            error!("Could not remove doc - {:?}", err);
            return;
        }
    }

    let _res = interaction
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.embed(|embed| {
                        embed
                            .title("Changes reverted")
                            .description(
                                "The connection that you added has been removed from the database.",
                            )
                            .field("User:", format!("<@{}>\n{}", &user_id, &user_id), false)
                            .field("Account:", account_type, false)
                            .field("Account ID:", account_id, false)
                            .footer(|footer| footer.text("Powered by Open/Alt.ID"))
                    })
                })
        })
        .await;
}

pub async fn register(ctx: &Context) {
    if let Err(err) = Command::create_global_application_command(&*ctx.http, |command| {
        command
            .name("addconnection")
            .description("Manually add connections to the database")
            .create_option(|opt| {
                opt.name("user")
                    .description("The user you are verifying")
                    .kind(CommandOptionType::User)
                    .required(true)
            })
            .create_option(|opt| {
                opt.name("account_type")
                    .description("The type of account you are adding (Twitch, YouTube, etc...)")
                    .kind(CommandOptionType::String)
                    .required(true)
            })
            .create_option(|opt| {
                opt.name("account_id")
                    .description("The ID of the account you are verifying")
                    .kind(CommandOptionType::String)
                    .required(true)
            })
    })
    .await
    {
        error!("Could not register verify command! {}", err.to_string());
        panic!()
    }
}

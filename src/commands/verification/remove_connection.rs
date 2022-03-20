use crate::commands::common::permissions_check::check_if_mod;
use crate::commands::common::slash_commands;
use crate::commands::common::{
    interaction_error::{interaction_error, interaction_error_comp},
    permissions_check::check_if_mod_comp,
};
use mongodb::bson::{doc, Document};
use mongodb::*;
use serenity::model::interactions::application_command::{
    ApplicationCommand, ApplicationCommandInteraction, ApplicationCommandOptionType,
};
use serenity::model::interactions::message_component::ButtonStyle;
use serenity::model::interactions::InteractionResponseType;
use serenity::model::user::User;
use serenity::{
    client::Context, model::interactions::message_component::MessageComponentInteraction,
};
use tracing::*;

pub async fn command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    mongo_client: &Client,
) {
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
    let collection: Collection<Document> = mongo_client.database("bot").collection("accounts");
    let delete_res = collection
        .delete_many(
            doc! {
                "user_ID": &user_id,
                "account_type": &account_type,
                "account_id": &account_id,
            },
            None,
        )
        .await;

    if delete_res.is_err() {
        super::super::common::interaction_error::interaction_error(
            "Could not remove account from database.",
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
                        .create_embed(|embed| {
                            embed
                                .title("Connection removed from database")
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
                                            "UndoRemoveConnection:{}:{}:{}",
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
    match check_if_mod_comp(&ctx, &interaction, &mongo_client).await {
        Ok(is_mod) => {
            if !is_mod {
                return;
            }
            {
                interaction_error_comp("You must be a mod to use this command.", interaction, ctx)
                    .await;
            }
        }
        Err(err) => {
            warn!("{}", err);
            interaction_error_comp(err, &interaction, ctx).await;
            return;
        }
    }

    let ids_split: Vec<&str> = interaction.data.custom_id.split(":").collect();

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

    let account_collection: Collection<Document> =
        mongo_client.database("bot").collection("accounts");
    let delete_res = account_collection
        .insert_one(
            doc! {
                "user_ID": &user_id,
                "account_type": &account_type,
                "account_id": &account_id,
            },
            None,
        )
        .await;

    match delete_res {
        Ok(doc) => debug!("{:?}", doc),
        Err(err) => {
            error!("Could not reinsert doc - {:?}", err);
            return;
        }
    }

    let _res = interaction
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.create_embed(|embed| {
                        embed
                            .title("Changes reverted")
                            .description("The connection was added back into the database.")
                            .field(
                                "User:",
                                format!("<@{}>\n{}", &account_id, &account_id),
                                false,
                            )
                            .field("Account:", account_type, false)
                            .field("Account ID:", account_id, false)
                            .footer(|footer| footer.text("Powered by Open/Alt.ID"))
                    })
                })
        })
        .await;
}

pub async fn register(ctx: &Context) {
    if let Err(err) = ApplicationCommand::create_global_application_command(&*ctx.http, |command| {
        command
            .name("removeconnection")
            .description("Manually remove connections to the database")
            .create_option(|opt| {
                opt.name("user")
                    .description("The user you are removing")
                    .kind(ApplicationCommandOptionType::User)
                    .required(true)
            })
            .create_option(|opt| {
                opt.name("account_type")
                    .description("The type of account you are removing (Twitch, YouTube, etc...)")
                    .kind(ApplicationCommandOptionType::String)
                    .required(true)
            })
            .create_option(|opt| {
                opt.name("account_id")
                    .description("The ID of the account you are removing")
                    .kind(ApplicationCommandOptionType::String)
                    .required(true)
            })
    })
    .await
    {
        error!("Could not register verify command! {}", err.to_string());
        panic!()
    }
}

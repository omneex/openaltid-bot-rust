use crate::commands::common::interaction_error::interaction_error;
use crate::dbmodels::guild::Guild as GuildDoc;
use chrono::Duration;
use chrono::Utc;
use mongodb::bson::doc;
use rand::distributions;
use rand::thread_rng;
use rand::Rng;
use redis::AsyncCommands;
use redis::RedisResult;
use redis::Value;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::application::interaction::MessageFlags;
use serenity::model::prelude::component::ButtonStyle;
use serenity::model::prelude::interaction::{application_command::*, InteractionResponseType};
use serenity::prelude::Context;
use serenity::utils::Colour;
use std::env;
use tracing::debug;
use tracing::{error, info, instrument, warn};

#[allow(unused)]
#[instrument(skip(ctx, mongo_client, redis_conn))]
pub async fn command(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    mongo_client: &mongodb::Client,
    mut redis_conn: &mut redis::aio::MultiplexedConnection,
) {
    let guild_id = match &command.guild_id {
        Some(id) => id.0,
        None => {
            interaction_error("This must be run in a guild.", command, ctx);
            return;
        }
    };

    let min_time = Utc::now() - Duration::days(137);

    // get guild settings from mongodb
    // if the server has no verification role set, log an error and return.
    let guild_doc_opt: Option<GuildDoc> = match mongo_client
        .database("botdb")
        .collection("guilds")
        .find_one(doc! {"guild_ID": guild_id.to_string()}, None)
        .await
    {
        Ok(col_opt) => col_opt,
        Err(err) => {
            error!("{:?}", err);
            return;
        }
    };
    // debug!("{:?}", guild_doc_opt);

    // Try to extract the guild doc from the option.
    let guild_doc: GuildDoc = match guild_doc_opt {
        None => {
            error!("Could not retrieve guild - guild_doc_opt was None");
            return;
        }
        Some(doc) => doc,
    };

    let num_days = match i64::try_from(guild_doc.verification_age) {
        Ok(num) => num,
        Err(err) => {
            error!("Could not convert u64 to i64 - {:?}", err);
            interaction_error(
                "This server's database is not properly configured, failed to convert age.",
                command,
                ctx,
            )
            .await;
            return;
        }
    };

    let time_now = Utc::now();

    let min_time = time_now - Duration::days(num_days);
    let mut member_of_command = match &command.member {
        Some(mem) => mem.clone(),
        None => return,
    };

    debug!(
        "time_now: {:?}, min_time: {:?}, created_at: {:?}",
        time_now,
        min_time,
        member_of_command.user.id.created_at()
    );

    if member_of_command.user.id.created_at().unix_timestamp() < min_time.timestamp() {
        let verification_role_id: u64 = match guild_doc.verification_role_ID.parse() {
            Ok(num) => num,
            Err(err) => {
                error!(
                    "Could not parse number from verification_role_ID - {:?}",
                    err
                );
                return;
            }
        };

        let channel_id: u64 = match guild_doc.verification_logs_channel_ID.parse() {
            Ok(num) => num,
            Err(err) => {
                error!(
                    "Could not parse number from verification_logs_channel_ID - {:?}",
                    err
                );
                return;
            }
        };

        let channel = match ctx.http.get_channel(channel_id).await {
            Ok(chn) => chn,
            Err(err) => {
                error!("{:?}", err);
                return;
            }
        };

        match &member_of_command
            .add_role(&ctx.http, verification_role_id)
            .await
        {
            Ok(_) => {
                debug!(
                    "Added role {} to user {}",
                    verification_role_id, member_of_command.user.id.0
                );
                let res = channel
                    .id()
                    .send_message(&ctx.http, |message| {
                        message.embed(|embed| {
                            embed.title("User Verified");
                            embed.color(Colour::BLUE);
                            embed.description("Above min age, automatically skipped verification.");
                            embed.timestamp(Utc::now());
                            embed.author(|author| {
                                author.name("Open/Alt.ID Logs");
                                author.url("https://github.com/omneex/OpenAltID");
                                author
                            });
                            embed.field(
                                "User Mention",
                                format!("<@{}>", &member_of_command.user.id.0),
                                false,
                            );
                            embed.field(
                                "User ID",
                                format!("{}", &member_of_command.user.id.0),
                                false,
                            );
                            embed.footer(|footer| {
                                footer.text("Powered by Open/Alt.ID");
                                footer
                            });
                            embed
                        })
                    })
                    .await;

                match res {
                    Ok(_) => {
                        // debug!("Embed message was sent successfully.")
                    }
                    Err(err) => {
                        warn!("Could not send embed - {:?}", err)
                    }
                }

                let _res = command.create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message.embed(|embed| {
                                embed
                                    .title("Auto-Verified")
                                    .description("Your account is above the min age, so you automatically skipped verification.")
                                    .footer(|footer| {
                                        footer.text("Powered by Open/Alt.ID")
                                    })
                            });
                            message.flags(MessageFlags::EPHEMERAL)
                        })
                }).await;
            }
            Err(err) => {
                error!("Could not add role to user during verification - {:?}", err);

                let res = channel.id().send_message(&ctx.http, |message| {
                    message.embed(|embed| {
                        embed.title("Error during verification!");
                        embed.color(Colour::DARK_RED);
                        embed.description("The role could not be added to the user and will need to be added manually.\n\n The user did however pass verification successfully. (auto-verified due to account age)");
                        embed.timestamp(Utc::now());
                        embed.author(|author| {
                            author.name("Open/Alt.ID Logs");
                            author.url("https://github.com/omneex/OpenAltID");
                            author
                        });
                        embed.field("Error Message", format!("{:?}", err), false);
                        embed.field("User Mention", format!("<@{}>", &member_of_command.user.id.0), false);
                        embed.field("User ID", format!("{}", &member_of_command.user.id.0), false);
                        embed.footer(|footer| {
                            footer.text("Powered by Open/Alt.ID");
                            footer
                        });
                        embed
                    })
                }).await;

                match res {
                    Ok(_) => {
                        // debug!("Embed message was sent successfully.")
                    }
                    Err(err) => {
                        warn!("Could not send embed - {:?}", err)
                    }
                }
            }
        }

        return;
    }

    let mut inserted = false;
    let mut rand_string: String = "".to_string();
    loop {
        rand_string = thread_rng()
            .sample_iter(&distributions::Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();

        let res: RedisResult<Value> = redis_conn.get(&rand_string).await;

        match res {
            Ok(val) => {
                // A value was found
                match val {
                    Value::Nil => break,
                    _ => {
                        info!("Dup key found, re-rolling");
                        continue;
                    }
                }
                debug!("The value for {} on redis was {:?}", rand_string, val);
                break;
            }
            Err(err) => {
                // An error occured
                error!("REDIS ERROR: {:?}", err);
                break;
            }
        }
    }
    let frontend_host = env::var("FRONTEND_HOST").expect("Need a frontend host in env.");
    let verification_link: String = format!("{}/verify?code={}", frontend_host, rand_string);

    let res: RedisResult<Value> = redis_conn
        .set_ex(
            format!("uuid:{}", rand_string),
            format!("{}:{}", command.user.id.0, guild_id),
            (15 * 60) as usize,
        )
        .await;
    debug!("Result from setting value - {:?}", res);

    let _res = command.create_interaction_response(&ctx.http, |response| {
        response
            .kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|message| {
                message.embed(|embed| {
                    embed
                        .title("Verification Initiated")
                        .description("Please follow the link below to start the verification process.\n\nYou must connect one or more of the supported accounts. The more you add the more likely you are to be verified.\n\nThis link will only stay valid for 15 minutes, after that you will need to use the command again.")
                        .field("Verification Link", verification_link.as_str(), false)
                        .field("Supported Accounts", "\nTwitch\nTwitter\nReddit\nYouTube", false)
                        .footer(|footer| {
                            footer.text("Powered by Open/Alt.ID")
                        })
                });

                message.components(|components| {
                    components.create_action_row(|row| {
                        row.create_button(|button| {
                            button
                                .style(ButtonStyle::Link)
                                .label("Click Here to Verify")
                                .url(verification_link.as_str())
                        });
                        row.create_button(|button| {
                            button
                                .style(ButtonStyle::Link)
                                .label("Read the privacy policy")
                                .url("https://verify.holoen.fans/privacy")
                        })
                    });

                    components.create_action_row(|row| {
                        row.create_button(|button| {
                            button
                                .style(ButtonStyle::Danger)
                                .label("I need help!")
                                .custom_id("HelpButton")
                        })
                    })
                });
                message.flags(MessageFlags::EPHEMERAL)
            })
    }).await;
    let channel_id = match guild_doc.verification_logs_channel_ID.parse::<u64>() {
        Ok(num) => num,
        Err(err) => {
            warn!("Could not parse int from verification channel logs");
            return;
        }
    };

    let logs_channel = match ctx.http.get_channel(channel_id).await {
        Ok(chn) => chn,
        Err(err) => {
            error!("Could not retrieve channel. ID from {}", channel_id);
            return;
        }
    };
    let member_obj = match &command.member {
        None => return,
        Some(mem) => mem,
    };

    logs_channel
        .id()
        .send_message(&ctx.http, |message| {
            message.embed(|embed| {
                embed.title("Verification Started");
                embed.color(Colour::GOLD);
                embed.description("The user has initiated verification,");
                embed.timestamp(Utc::now());
                embed.thumbnail(&member_obj.face());
                embed.author(|author| {
                    author.name("Open/Alt.ID Logs");
                    author.url("https://github.com/omneex/OpenAltID");
                    author
                });
                embed.field(
                    "User Mention",
                    format!("<@{}>", member_obj.user.id.0),
                    false,
                );
                embed.field("User ID", member_obj.user.id.0.to_string(), false);
                embed.field("Link Provided", verification_link.to_string(), false);
                embed.field(
                    "Expires In",
                    format!("<t:{}:R>", (time_now + Duration::minutes(15)).timestamp()),
                    false,
                );
                embed.footer(|footer| {
                    footer.text("Powered by Open/Alt.ID");
                    footer
                });
                embed
            })
        })
        .await;
}
pub async fn help_callback(
    ctx: &Context,
    interaction: &MessageComponentInteraction,
    _mongo_client: &mongodb::Client,
) {
    info!("Creating response...");
    let _res = interaction.create_interaction_response(&ctx.http, |response| {
        response
            .kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|message| {
                message.embed(|embed| {
                    embed
                        .title("Sorry you're having troubles!")
                        .description("Follow this check list to ensure you can verify:")
                        .field("1.", "Make sure you have connected supported accounts to Discord.", false)
                        .field("2.", "Ensure you are using the oldest accounts you have, and more is better.", false)
                        .field("3.", "Ensure you wait for the countdown to end before clicking verify.", false)
                        .field("Finally", "If you are still having issues, contact server staff for more help.", false)
                        .footer(|footer| {
                            footer.text("Powered by Open/Alt.ID")
                        })
                });
                message.flags(MessageFlags::EPHEMERAL)
            })
    }).await;
}

pub async fn register(ctx: &Context) {
    if let Err(err) = Command::create_global_application_command(&*ctx.http, |command| {
        command.name("verify").description("Verify")
    })
    .await
    {
        error!("Could not register verify command! {}", err.to_string());
        panic!()
    }
}

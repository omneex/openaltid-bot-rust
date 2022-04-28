use crate::dbmodels::guild::Guild as GuildDoc;
use chrono::Utc;
use mongodb::bson::doc;
use redis::{AsyncCommands, RedisError};
use serenity::{client::Context, utils::Colour};
use std::{env, sync::Arc};
use tracing::*;

pub async fn check_redis(
    ctx: Arc<Context>,
    mongo_client: Arc<mongodb::Client>,
    redis_client: Arc<redis::Client>,
) {
    // Format for completed verification keys is: complete:{userid}:{guildid}
    // Value must be either 'true' or 'false'
    let mut conn = match redis_client.get_async_connection().await {
        Ok(conn) => conn,
        Err(err) => {
            error!("Error getting connection to redis - {:?}", err);
            return;
        }
    };
    let is_debug = if let Ok(val) = env::var("DEBUG") {
        val.parse().unwrap_or(false)
    } else {
        false
    };

    if is_debug {
        if let Err(err) = conn
            .set::<String, String, String>(
                "complete:155149108183695360:416407744246054912:3".to_string(),
                "true:400:350".to_string(),
            )
            .await
        {
            error!("1 {:?}", err);
        };
        if let Err(err) = conn
            .set::<String, String, String>(
                "complete:155149108183695360:416407744246054912:1".to_string(),
                "false:32:350".to_string(),
            )
            .await
        {
            error!("2 {:?}", err);
        };
        if let Err(err) = conn
            .set::<String, String, String>(
                "complete:155149108183695360:416407744246054912:2".to_string(),
                "error:this is just a test".to_string(),
            )
            .await
        {
            error!("3 {:?}", err);
        };
        if let Err(err) = conn
            .set::<String, String, String>("failed".to_string(), "true".to_string())
            .await
        {
            error!("4 {:?}", err);
        };

        if let Err(err) = conn
            .set::<String, String, String>("failed".to_string(), "true".to_string())
            .await
        {
            error!("5 {:?}", err);
        };
        let res: Result<String, RedisError> = conn
            .get("complete:179780264761884672:416407744246054912")
            .await;
        debug!("{:?}", res);
    }

    let keys = match conn.scan_match("complete*").await {
        Ok(mut iter) => {
            let mut keys: Vec<String> = vec![];
            while let Some(key) = iter.next_item().await {
                keys.push(key);
            }
            debug!("{:?}", keys);
            debug!("Keys have been extracted from the scan.");
            keys
        }
        Err(err) => {
            error!("Redis error in scan - {:?}", err);
            return;
        }
    };

    for key in keys {
        debug!("KEY FOUND FROM SCAN: {}", &key);
        // get the value from redis

        let val = match conn.get::<String, String>(key.clone().to_string()).await {
            Ok(val) => val,
            Err(err) => {
                error!(
                    "Failed to get value from db with a key {} from iterator - {:?}",
                    key, err
                );
                continue;
            }
        };

        debug!("Value from iter key ({}) - {}", &key, val);

        // check for "true*"
        if val.starts_with("true") {
            debug!("Value starts with true.");
            // split key on ':'
            let key_split = key.split(':').collect::<Vec<&str>>().clone();
            debug!("{:?}", key_split);
            // user_id is index 1
            let user_id = match key_split.get(1) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid key found when splitting - {}", key);
                    return;
                }
            };
            debug!("{:?}", user_id);
            // guild_id is index 2
            let guild_id = match key_split.get(2) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid val found when splitting - {}", val);
                    return;
                }
            };
            debug!("{:?}", guild_id);

            // split val on ':'
            // score is index 1
            // minscore is index 2
            let val_split: Vec<&str> = val.split(':').collect();
            debug!("{:?}", val_split);

            let score = match val_split.get(1) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid val found when splitting - {}", val);
                    return;
                }
            };

            let minscore = match val_split.get(2) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid val found when splitting - {}", val);
                    return;
                }
            };

            // get guild obj
            let guild_id: u64 = match guild_id.parse() {
                Ok(num) => num,
                Err(err) => {
                    error!(
                        "Could not parse number from verification_logs_channel_ID - {:?}",
                        err
                    );
                    return;
                }
            };
            debug!("{:?}", guild_id);
            let _guild_obj = match ctx.http.get_guild(guild_id).await {
                Ok(chn) => chn,
                Err(err) => {
                    error!("Error getting guild - {:?}", err);
                    return;
                }
            };

            // get member obj
            let user_id: u64 = match user_id.parse() {
                Ok(num) => num,
                Err(err) => {
                    error!(
                        "Could not parse number from verification_logs_channel_ID - {:?}",
                        err
                    );
                    return;
                }
            };
            debug!("{:?}", user_id);
            let mut member_obj = match ctx.http.get_member(guild_id, user_id).await {
                Ok(mem) => mem,
                Err(err) => {
                    error!("Error getting member obj - {:?}", err);
                    return;
                }
            };

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
                    error!("Mongo error - {:?}", err);
                    return;
                }
            };
            debug!("{:?}", guild_doc_opt);

            // Try to extract the guild doc from the option.
            let guild_doc: GuildDoc = match guild_doc_opt {
                None => {
                    error!("Could not retrieve guild - guild_doc_opt was None");
                    return;
                }
                Some(doc) => doc,
            };
            debug!("{:?}", guild_doc);

            // get the role obj from the guild settings
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
            // add the role to the user

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
            debug!("{:?}", channel_id);
            let channel = match ctx.http.get_channel(channel_id).await {
                Ok(chn) => chn,
                Err(err) => {
                    error!("Getting channel - {:?}", err);
                    return;
                }
            };

            match member_obj.add_role(&ctx.http, verification_role_id).await {
                Ok(_) => {
                    debug!("Added role {} to user {}", verification_role_id, user_id)
                }
                Err(err) => {
                    error!("Could not add role to user during verification - {:?}", err);

                    let res = channel.id().send_message(&ctx.http, |message| {
                        message.embed(|embed| {
                            embed.title("Error during verification!");
                            embed.color(Colour::DARK_RED);
                            embed.description("The role could not be added to the user and will need to be added manually.\n\n The user did however pass verification successfully.");
                            embed.timestamp(Utc::now());
                            embed.thumbnail(&member_obj.face());
                            embed.author(|author| {
                                author.name("Open/Alt.ID Logs");
                                author.url("https://github.com/omneex/OpenAltID");
                                author
                            });
                            embed.field("Error Message", format!("{:?}", err), false);
                            embed.field("User Mention", format!("<@{}>", user_id), false);
                            embed.field("User ID", format!("{}", user_id), false);
                            embed.field("Score", format!("**{}** / {}", score, minscore), false);
                            embed.footer(|footer| {
                                footer.text("Powered by Open/Alt.ID");
                                footer
                            });
                            embed
                        })
                    }).await;

                    match res {
                        Ok(_) => {
                            debug!("Embed message was sent successfully.")
                        }
                        Err(err) => {
                            warn!("Could not send embed - {:?}", err)
                        }
                    }
                }
            }

            // delete the key from redis
            // log the info
            let _del_res: u16 = match conn.del(&key).await {
                Ok(val) => val,
                Err(err) => {
                    error!("Failed to delete key: {} - {}", key, err);
                    return;
                }
            };
            // check if the verification logs channel is set up
            // if it is set up then send the log info to the channel in an embed
            // log that the user encountered an error with the reason
            info!(
                "User: {} was verified in {} Score: {} / {}",
                user_id, guild_id, score, minscore
            );
            // check if the server has a logs channel
            // if it is set up then send the log info to the channel in an embed
            debug!(
                "Will now send the info to the logs channel which is {} with the Score: {} / {}.",
                channel_id, score, minscore
            );
            let res = channel
                .id()
                .send_message(&ctx.http, |message| {
                    message.embed(|embed| {
                        embed.title("Verification Passed");
                        embed.color(Colour::BLUE);
                        embed.description("The user passed verification.");
                        embed.timestamp(Utc::now());
                        embed.thumbnail(&member_obj.face());
                        embed.author(|author| {
                            author.name("Open/Alt.ID Logs");
                            author.url("https://github.com/omneex/OpenAltID");
                            author
                        });
                        embed.field("User Mention", format!("<@{}>", user_id), false);
                        embed.field("User ID", format!("{}", user_id), false);
                        embed.field("Score", format!("**{}** / {}", score, minscore), false);
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
                    debug!("Embed message was sent successfully.")
                }
                Err(err) => {
                    warn!("Could not send embed - {:?}", err)
                }
            }
        }
        // check for "false*"
        else if val.starts_with("false") {
            debug!("Value starts with false.");
            // split key on ':'
            let key_split = key.split(':').collect::<Vec<&str>>().clone();
            debug!("{:?}", key_split);
            // user_id is index 1
            let user_id = match key_split.get(1) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid key found when splitting - {}", key);
                    return;
                }
            };
            debug!("{:?}", user_id);
            // guild_id is index 2
            let guild_id = match key_split.get(2) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid val found when splitting - {}", val);
                    return;
                }
            };
            debug!("{:?}", guild_id);

            // split val on ':'
            // score is index 1
            // minscore is index 2
            let val_split: Vec<&str> = val.split(':').collect();
            debug!("{:?}", val_split);

            let score = match val_split.get(1) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid val found when splitting - {}", val);
                    return;
                }
            };

            let minscore = match val_split.get(2) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid val found when splitting - {}", val);
                    return;
                }
            };

            // get member obj
            let user_id: u64 = match user_id.parse() {
                Ok(num) => num,
                Err(err) => {
                    error!(
                        "Could not parse number from verification_logs_channel_ID - {:?}",
                        err
                    );
                    return;
                }
            };
            let guild_id: u64 = match guild_id.parse() {
                Ok(num) => num,
                Err(err) => {
                    error!(
                        "Could not parse number from verification_logs_channel_ID - {:?}",
                        err
                    );
                    return;
                }
            };
            debug!("{:?}", user_id);
            let member_obj = match ctx.http.get_member(guild_id, user_id).await {
                Ok(mem) => mem,
                Err(err) => {
                    error!("Cant get member_obj - {:?} - Removing from the queue!", err);
                    let del_res: u16 = match conn.del(&key).await {
                        Ok(val) => val,
                        Err(err) => {
                            error!("Failed to delete key: {} - {}", key, err);
                            return;
                        }
                    };
                    debug!("{:?}", del_res);
                    return;
                }
            };

            // get guild settings from mongodb
            // if the server has no verification role set, log an error and return.
            // get guild settings from mongodb
            let guild_doc_opt: Option<GuildDoc> = match mongo_client
                .database("botdb")
                .collection("guilds")
                .find_one(doc! {"guild_ID": guild_id.to_string()}, None)
                .await
            {
                Ok(col_opt) => col_opt,
                Err(err) => {
                    error!("Error gettting doc opt - {:?}", err);
                    return;
                }
            };
            debug!("{:?}", guild_doc_opt);

            // Try to extract the guild doc from the option.
            let guild_doc: GuildDoc = match guild_doc_opt {
                None => {
                    error!("Could not retrieve guild - guild_doc_opt was None");
                    return;
                }
                Some(doc) => doc,
            };
            debug!("{:?}", guild_doc);

            // delete the key from redis
            // log the info
            let del_res: u16 = match conn.del(&key).await {
                Ok(val) => val,
                Err(err) => {
                    error!("Failed to delete key: {} - {}", key, err);
                    return;
                }
            };
            debug!("{:?}", del_res);

            // check if the verification logs channel is set up
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
            debug!("{:?}", channel_id);
            let channel = match ctx.http.get_channel(channel_id).await {
                Ok(chn) => chn,
                Err(err) => {
                    error!("Error getting channel - {:?}", err);
                    return;
                }
            };
            debug!("{:?}", channel_id);
            // if it is set up then send the log info to the channel in an embed
            // log that the user encountered an error with the reason
            info!(
                "User: {} was NOT verified in {} Score: {} / {}",
                user_id, guild_id, score, minscore
            );

            // check if the server has a logs channel
            // if it is set up then send the log info to the channel in an embed
            debug!(
                "Will now send the info to the logs channel which is {} with the Score: {} / {}.",
                channel_id, score, minscore
            );
            let res = channel
                .id()
                .send_message(&ctx.http, |message| {
                    message.embed(|embed| {
                        embed.title("Verification Failed");
                        embed.color(Colour::ORANGE);
                        embed.description("The user did not pass verification.");
                        embed.timestamp(Utc::now());
                        embed.thumbnail(&member_obj.face());
                        embed.author(|author| {
                            author.name("Open/Alt.ID Logs");
                            author.url("https://github.com/omneex/OpenAltID");
                            author
                        });
                        embed.field("User Mention", format!("<@{}>", user_id), false);
                        embed.field("User ID", user_id.to_string(), false);
                        embed.field("Score", format!("**{}** / {}", score, minscore), false);
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
                    debug!("Embed message was sent successfully.")
                }
                Err(err) => {
                    warn!("Could not send embed - {:?}", err)
                }
            }
        }
        // check for "error*"
        else if val.starts_with("error") {
            debug!("Value starts with error.");
            // split key on ':'
            let key_split = key.split(':').collect::<Vec<&str>>().clone();
            debug!("{:?}", key_split);
            // user_id is index 1
            let user_id = match key_split.get(1) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid key found when splitting - {}", key);
                    return;
                }
            };
            debug!("{:?}", user_id);
            // guild_id is index 2
            let guild_id = match key_split.get(2) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid val found when splitting - {}", val);
                    return;
                }
            };
            debug!("{:?}", guild_id);

            // split val on ':'
            // reason is index 1
            let val_split: Vec<&str> = val.split(':').collect();
            debug!("{:?}", val_split);
            // user_id is index 1
            let reason = match val_split.get(1) {
                Some(val) => <&str>::clone(val),
                None => {
                    error!("Invalid val found when splitting - {}", val);
                    return;
                }
            };

            // get member obj
            let user_id: u64 = match user_id.parse() {
                Ok(num) => num,
                Err(err) => {
                    error!(
                        "Could not parse number from verification_logs_channel_ID - {:?}",
                        err
                    );
                    return;
                }
            };
            let guild_id: u64 = match guild_id.parse() {
                Ok(num) => num,
                Err(err) => {
                    error!(
                        "Could not parse number from verification_logs_channel_ID - {:?}",
                        err
                    );
                    return;
                }
            };
            debug!("{:?}", user_id);
            let member_obj = match ctx.http.get_member(guild_id, user_id).await {
                Ok(mem) => mem,
                Err(err) => {
                    error!("Error getting member - {:?}", err);
                    return;
                }
            };

            debug!("{:?}", reason);
            // get guild settings from mongodb
            let guild_doc_opt: Option<GuildDoc> = match mongo_client
                .database("botdb")
                .collection("guilds")
                .find_one(doc! {"guild_ID": guild_id.to_string()}, None)
                .await
            {
                Ok(col_opt) => col_opt,
                Err(err) => {
                    error!("Error getting doc - {:?}", err);
                    return;
                }
            };
            debug!("{:?}", guild_doc_opt);

            // Try to extract the guild doc from the option.
            let guild_doc: GuildDoc = match guild_doc_opt {
                None => {
                    error!("Could not retrieve guild - guild_doc_opt was None");
                    return;
                }
                Some(doc) => doc,
            };
            debug!("{:?}", guild_doc);

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
            debug!("{:?}", channel_id);
            let channel = match ctx.http.get_channel(channel_id).await {
                Ok(chn) => chn,
                Err(err) => {
                    error!("Error getting channel - {:?}", err);
                    return;
                }
            };

            // delete the key from redis
            let del_res: u16 = match conn.del(&key).await {
                Ok(val) => val,
                Err(err) => {
                    error!("Failed to delete key: {} - {}", key, err);
                    return;
                }
            };
            debug!("{:?}", del_res);
            // log that the user encountered an error with the reason
            info!(
                "User: {} was NOT verified in {} Reason: {}",
                user_id, guild_id, reason
            );
            // check if the server has a logs channel
            // if it is set up then send the log info to the channel in an embed
            debug!(
                "Will now send the info to the logs channel which is {} with the reason of '{}'.",
                channel_id, reason
            );

            let res = channel
                .id()
                .send_message(&ctx.http, |message| {
                    message.embed(|embed| {
                        embed.title("Verification Failed");
                        embed.color(Colour::RED);
                        embed.description("The user could not be verified.");
                        embed.timestamp(Utc::now());
                        embed.thumbnail(&member_obj.face());
                        embed.author(|author| {
                            author.name("Open/Alt.ID Logs");
                            author.url("https://github.com/omneex/OpenAltID");
                            author
                        });
                        embed.field("User Mention", format!("<@{}>", user_id), false);
                        embed.field("User ID", user_id.to_string(), false);
                        embed.field("Reason", format!("__{}__", reason), false);
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
                    debug!("Embed message was sent successfully.")
                }
                Err(err) => {
                    warn!("Could not send embed - {:?}", err)
                }
            }
        } else {
            warn!("Value did not start with a supported string. {}", val)
        }
    }
}

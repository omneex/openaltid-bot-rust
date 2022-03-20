use serenity::prelude::*;
use mongodb::*;
use mongodb::bson::doc;
use mongodb::options::IndexOptions;
use tracing::*;
use crate::{get_collection, get_db};
use crate::dbmodels::guild::{GuildSettings, Guild};

#[instrument(skip(ctx, client))]
pub async fn insert_guilds(ctx: &Context, client: &mongodb::Client) -> Result<(), String> {
    let db = get_db(client, "bot").await;
    let col: Collection<Guild> = get_collection(&db, "guilds", None).await;
    let guilds = ctx.cache.guilds().await;
    for guild in guilds {
        info!("Inserting ({}) into MongoDB", guild.0);
        let res = col.insert_one(Guild {
            guild_ID: guild.0.to_string(),
            mod_channel_ID: "0".to_string(),
            verification_channel_ID: "0".to_string(),
            verification_role_ID: "0".to_string(),
            mod_role_ID: "0".to_string(),
            prefix_string: "~".to_string(),
            verification_age: 0,
            enabled: false,
            verify_on_screening: false,
            verification_logs_channel_ID: "0".to_string(),
            guild_settings: GuildSettings {
                zero_point: 0,
                difficulty_addition: 0,
                mfa_bonus: 0,
                premium_bonus: 0,
                preferred_num_of_accounts: 0
            }
        }, None).await;

        if let Err(err) = res {
            return Err(format!("{:?}", err));
        }

        let model = IndexModel::builder().keys(doc! {"guild_ID": 1}).options(IndexOptions::builder().unique(true).build()).build();

        let res = col.create_index(model, None).await;

        match res {
            Ok(_) => {}
            Err(e) => {
                error!("{:?}", e)
            }
        }
    }
    Ok(())
}
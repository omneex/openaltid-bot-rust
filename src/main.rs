mod application_commands;
mod commands;
mod dbmodels;
mod mongo_conn;
mod redis_check_loop;
mod startup;

use crate::{
    mongo_conn::{get_collection, get_db, get_mongo_client},
    startup::insert_guilds,
};
use redis_check_loop::check_redis;
use serenity::{async_trait, framework::StandardFramework, model::prelude::*, prelude::*};
use std::{
    env,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tracing::*;

struct Handler {
    mongodb_client: mongodb::Client,
    redis_client: redis::Client,
    is_loop_running: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    // We use the cache_ready event just in case some cache operation is required in whatever use
    // case you have for this.
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let ctx = Arc::new(ctx);

        let mongo_conn_str = env::var("MONGO_CONN_STR").expect("Need a MongoDB connection string.");
        let mongodb_client = match get_mongo_client(mongo_conn_str.as_str()).await {
            Ok(client) => client,
            Err(err) => {
                panic!("Could not get mongoDB client. {:?}", err)
            }
        };

        let redis_url = env::var("REDIS_HOST").expect("Need a Redis connection string.");
        let redis_client = match redis::Client::open(format!("redis://{}/", redis_url)) {
            Ok(client) => client,
            Err(err) => {
                panic!("Could not get redis client. {:?}", err)
            }
        };

        let mongo_client = Arc::new(mongodb_client);
        let redis_client = Arc::new(redis_client);

        if !self.is_loop_running.load(Ordering::Relaxed) {
            let ctx1 = Arc::clone(&ctx);
            let mongo_client1 = Arc::clone(&mongo_client);
            let redis_client1 = Arc::clone(&redis_client);

            tokio::spawn(async move {
                loop {
                    check_redis(
                        Arc::clone(&ctx1),
                        Arc::clone(&mongo_client1),
                        Arc::clone(&redis_client1),
                    )
                    .await;
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });

            // Now that the loop is running, we set the bool to true
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        // let clear_commands = false;
        // if clear_commands {
        //     application_commands::clear(&ctx).await;
        // }

        let mongo_conn_str = env::var("MONGO_CONN_STR").expect("Need a MongoDB connection string.");
        let client = match get_mongo_client(mongo_conn_str.as_str()).await {
            Ok(client) => client,
            Err(_) => {
                panic!("Could not get mongoDB client.")
            }
        };
        if let Err(err) = insert_guilds(&ctx, &client).await {
            warn!("{:?}", err)
        }

        application_commands::register(&ctx).await;
    }

    async fn interaction_create(&self, _ctx: Context, _interaction: Interaction) {
        // If the interaction is an Application Command then name the interaction applicationCommand
        // and move on to the evaluate the block
        application_commands::handle_interactions(
            &_ctx,
            _interaction,
            &self.mongodb_client,
            &self.redis_client,
        )
        .await
    }
}

#[tokio::main]
async fn main() {
    // Initialize the tracing subscriber
    tracing_subscriber::fmt().json().init();
    info!("Starting the bot...");

    let framework = StandardFramework::new().configure(|c| c.prefix("~")); // set the bot's prefix to "~"

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("Missing token in env!");

    let application_id: u64 = env::var("APPLICATION_ID")
        .expect("Expected an application id in the environment")
        .parse()
        .expect("application id is not a valid id");

    let mongo_conn_str = env::var("MONGO_CONN_STR").expect("Need a MongoDB connection string.");
    let mongodb_client = match get_mongo_client(mongo_conn_str.as_str()).await {
        Ok(client) => client,
        Err(err) => {
            panic!("Could not get mongoDB client. {:?}", err)
        }
    };

    let redis_url = env::var("REDIS_HOST").expect("Need a MongoDB connection string.");
    let redis_client = match redis::Client::open(format!("redis://{}/", redis_url)) {
        Ok(client) => client,
        Err(e) => panic!("Could not get redis client. {}", e),
    };

    let handler = Handler {
        mongodb_client,
        redis_client,
        is_loop_running: AtomicBool::new(false),
    };

    let mut client = Client::builder(token)
        .event_handler(handler)
        .framework(framework)
        .application_id(application_id)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        error!("An error occurred while running the client: {:?}", why);
    }
}

use mongodb::options::{ClientOptions, CollectionOptions, ResolverConfig};
use mongodb::*;
pub async fn get_mongo_client(connection_str: &str) -> mongodb::error::Result<Client> {
    let client_options = ClientOptions::parse_with_resolver_config(connection_str, ResolverConfig::cloudflare()).await?;
    Client::with_options(client_options)
}

pub async fn get_db(client: &Client, db_name: &str) -> Database {
    client.database(db_name)
}

pub async fn get_collection<T>(
    db: &Database,
    collection_str: &str,
    options: Option<CollectionOptions>,
) -> Collection<T> {
    let col: Collection<T>;
    match options {
        None => col = db.collection(collection_str),
        Some(options) => col = db.collection_with_options(collection_str, options),
    }
    col
}

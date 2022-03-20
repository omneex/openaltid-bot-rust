use serde::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct GuildSettings {
    pub zero_point: i64,
    pub difficulty_addition: i64,
    pub mfa_bonus: i64,
    pub premium_bonus: i64,
    pub preferred_num_of_accounts: u8,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Guild {
    pub guild_ID: String,
    pub mod_channel_ID: String,
    pub verification_channel_ID: String,
    pub verification_role_ID: String,
    pub mod_role_ID: String,
    pub prefix_string: String,
    pub verification_age: u64,
    pub enabled: bool,
    pub verify_on_screening: bool,
    pub verification_logs_channel_ID: String,
    pub guild_settings: GuildSettings,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SocialMediaAccounts {
    pub account_type: String,
    pub account_ID: String,
    pub discord_ID: String,
}

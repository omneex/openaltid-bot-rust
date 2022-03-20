use serenity::model::channel::PartialChannel;
use serenity::model::guild::Role;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteractionDataOption, ApplicationCommandInteractionDataOptionValue,
};
use serenity::model::user::User;

pub async fn extract_vec(
    options: &[ApplicationCommandInteractionDataOption],
) -> Vec<(&str, ApplicationCommandInteractionDataOptionValue)> {
    let mut params: Vec<(&str, ApplicationCommandInteractionDataOptionValue)> = vec![];
    options.iter().for_each(|opt| {
        let value: Option<ApplicationCommandInteractionDataOptionValue> =
            opt.to_owned().resolved;
        match value {
            None => {}
            Some(value) => params.push((opt.name.as_str(), value)),
        }
    });
    params
}
#[allow(dead_code)]
pub async fn get_number(option_value: ApplicationCommandInteractionDataOptionValue) -> Option<f64> {
    let value: Option<f64> = match option_value {
        ApplicationCommandInteractionDataOptionValue::Number(num) => Some(num),
        _ => None,
    };
    value
}
#[allow(dead_code)]
pub async fn get_role(option_value: ApplicationCommandInteractionDataOptionValue) -> Option<Role> {
    let value: Option<Role> = match option_value {
        ApplicationCommandInteractionDataOptionValue::Role(role) => Some(role),
        _ => None,
    };
    value
}

pub async fn get_channel(
    option_value: ApplicationCommandInteractionDataOptionValue,
) -> Option<PartialChannel> {
    let value: Option<PartialChannel> = match option_value {
        ApplicationCommandInteractionDataOptionValue::Channel(chan) => Some(chan),
        _ => None,
    };
    value
}

pub async fn get_user(option_value: ApplicationCommandInteractionDataOptionValue) -> Option<User> {
    let value: Option<User> = match option_value {
        ApplicationCommandInteractionDataOptionValue::User(user, _) => Some(user),
        _ => None,
    };
    value
}
#[allow(dead_code)]
pub async fn get_bool(option_value: ApplicationCommandInteractionDataOptionValue) -> Option<bool> {
    let value: Option<bool> = match option_value {
        ApplicationCommandInteractionDataOptionValue::Boolean(bool) => Some(bool),
        _ => None,
    };
    value
}

pub async fn get_string(
    option_value: ApplicationCommandInteractionDataOptionValue,
) -> Option<String> {
    let value: Option<String> = match option_value {
        ApplicationCommandInteractionDataOptionValue::String(string) => Some(string),
        _ => None,
    };
    value
}
pub async fn get_int(option_value: ApplicationCommandInteractionDataOptionValue) -> Option<i64> {
    let value: Option<i64> = match option_value {
        ApplicationCommandInteractionDataOptionValue::Integer(i) => Some(i),
        _ => None,
    };
    value
}

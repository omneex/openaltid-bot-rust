
use serenity::model::prelude::{Role, PartialChannel};
use serenity::model::prelude::interaction::application_command::CommandDataOptionValue;
use serenity::model::{user::User, application::interaction::application_command::CommandDataOption};

pub async fn extract_vec(
    options: &[CommandDataOption],
) -> Vec<(&str, CommandDataOptionValue)> {
    let mut params: Vec<(&str, CommandDataOptionValue)> = vec![];
    options.iter().for_each(|opt| {
        let value: Option<CommandDataOptionValue> = opt.to_owned().resolved;
        match value {
            None => {}
            Some(value) => params.push((opt.name.as_str(), value)),
        }
    });
    params
}
#[allow(dead_code)]
pub async fn get_number(option_value: CommandDataOptionValue) -> Option<f64> {
    let value: Option<f64> = match option_value {
        CommandDataOptionValue::Number(num) => Some(num),
        _ => None,
    };
    value
}
#[allow(dead_code)]
pub async fn get_role(option_value: CommandDataOptionValue) -> Option<Role> {
    let value: Option<Role> = match option_value {
        CommandDataOptionValue::Role(role) => Some(role),
        _ => None,
    };
    value
}

pub async fn get_channel(
    option_value: CommandDataOptionValue,
) -> Option<PartialChannel> {
    let value: Option<PartialChannel> = match option_value {
        CommandDataOptionValue::Channel(chan) => Some(chan),
        _ => None,
    };
    value
}

pub async fn get_user(option_value: CommandDataOptionValue) -> Option<User> {
    let value: Option<User> = match option_value {
        CommandDataOptionValue::User(user, _) => Some(user),
        _ => None,
    };
    value
}
#[allow(dead_code)]
pub async fn get_bool(option_value: CommandDataOptionValue) -> Option<bool> {
    let value: Option<bool> = match option_value {
        CommandDataOptionValue::Boolean(bool) => Some(bool),
        _ => None,
    };
    value
}

pub async fn get_string(
    option_value: CommandDataOptionValue,
) -> Option<String> {
    let value: Option<String> = match option_value {
        CommandDataOptionValue::String(string) => Some(string),
        _ => None,
    };
    value
}
pub async fn get_int(option_value: CommandDataOptionValue) -> Option<i64> {
    let value: Option<i64> = match option_value {
        CommandDataOptionValue::Integer(i) => Some(i),
        _ => None,
    };
    value
}

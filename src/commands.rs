use serde::{Deserialize, Serialize};

use crate::interaction::{CommandType, Interaction, OptionType};

pub trait Command {
    fn handle(&self, interaction: Interaction);
    fn register(&self);
    fn delete(&self);
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationCommand {
    id: String,
    r#type: Option<CommandType>,
    application_id: String,
    guild_id: Option<String>,
    name: CommandName,
    description: String,
    options: Option<Vec<ApplicationCommandOption>>,
    default_member_permissions: Option<String>,
    dm_permission: Option<bool>,
    version: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum CommandName {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "entry")]
    Entry,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationCommandOption {
    r#type: OptionType,
    name: String,
    description: String,
    required: Option<bool>,
    min_length: Option<u32>,
    max_length: Option<u32>,
}

impl Command for ApplicationCommand {
    fn handle(&self, interaction: Interaction) {}
    fn register(&self) {}
    fn delete(&self) {}
}

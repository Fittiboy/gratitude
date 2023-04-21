use serde::{Deserialize, Serialize};
use worker::{console_debug, console_error, console_log, Env, Result};

use crate::interaction::{CommandName, CommandType, OptionType};
use crate::DiscordAPIClient;

pub async fn update(env: &Env, client: &mut DiscordAPIClient) {
    let application_id = env.var("DISCORD_APPLICATION_ID").unwrap().to_string();

    let mut registered = ApplicationCommand::registered(&application_id, client).await;
    let mut available = ApplicationCommand::globals(&application_id).unwrap();

    registered.retain(|c| !available.has(c));
    delete(&registered, client).await;

    available.retain(|c| !registered.has(c));
    register(&available, client).await;
}

pub async fn delete(commands: &Vec<ApplicationCommand>, client: &mut DiscordAPIClient) {
    for command in commands {
        command.delete(client).await;
    }
}

pub async fn register(commands: &Vec<ApplicationCommand>, client: &mut DiscordAPIClient) {
    for command in commands {
        command.register(client).await;
    }
}

trait HasCommand {
    fn has(&self, other: &ApplicationCommand) -> bool;
}

impl HasCommand for Vec<ApplicationCommand> {
    fn has(&self, other: &ApplicationCommand) -> bool {
        for command in self {
            if command.name == other.name {
                return true;
            }
        }
        false
    }
}

#[allow(dead_code)]
impl ApplicationCommand {
    pub fn globals(application_id: &str) -> Result<Vec<Self>> {
        Ok(vec![
            Self {
                name: CommandName::Help,
                application_id: application_id.to_string(),
                description: "Get some information about the bot!".into(),
                ..Default::default()
            },
            Self {
                name: CommandName::Start,
                application_id: application_id.to_string(),
                description: "Start receiving reminders from the bot!".into(),
                dm_permission: Some(true),
                ..Default::default()
            },
            Self {
                name: CommandName::Stop,
                application_id: application_id.to_string(),
                description: "Stop receiving reminders from the bot!".into(),
                dm_permission: Some(true),
                ..Default::default()
            },
            Self {
                name: CommandName::Entry,
                description: "Add an entry to your gratitude journal!".into(),
                options: Some(vec![ApplicationCommandOption {
                    r#type: OptionType::String,
                    name: "entry".into(),
                    description: "Something, anything, you are feeling grateful for!".into(),
                    required: Some(true),
                    min_length: Some(5),
                    max_length: Some(1000),
                }]),
                application_id: application_id.to_string(),
                dm_permission: Some(true),
                ..Default::default()
            },
        ])
    }

    pub async fn registered(application_id: &str, client: &mut DiscordAPIClient) -> Vec<Self> {
        let response = match client
            .get(&format!("applications/{}/commands", application_id))
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                console_error!("Couldn't find commands: {}", err);
                panic!();
            }
        };
        let commands = match response.error_for_status() {
            Ok(response) => {
                console_debug!("Response: {:#?}", response);
                response.json::<Vec<ApplicationCommand>>().await
            }
            Err(err) => {
                console_error!("Did not get expected response: {}", err);
                panic!();
            }
        };
        match commands {
            Ok(commands) => commands,
            Err(err) => {
                console_error!("Couldn't parse commands response: {}", err);
                panic!();
            }
        }
    }

    pub fn by_name(name: CommandName) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    pub async fn get_id(&self, client: &mut DiscordAPIClient) -> Option<String> {
        let commands = ApplicationCommand::registered(&self.application_id, client).await;
        match commands.iter().find(|command| command.name == self.name) {
            Some(command) => command.id.clone(),
            None => None,
        }
    }

    pub async fn register(&self, client: &mut DiscordAPIClient) -> Self {
        let response = match client
            .post(&format!("applications/{}/commands", self.application_id))
            .json(&CommandRegister::from(self))
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                console_error!("Couldn't register command: {}", err);
                panic!()
            }
        };
        let command = match response.error_for_status() {
            Ok(response) => {
                console_debug!("Response: {:#?}", response);
                response.json::<ApplicationCommand>().await
            }
            Err(err) => {
                console_error!("Did not get expected response: {}", err);
                panic!();
            }
        };
        match command {
            Ok(command) => command,
            Err(err) => {
                console_error!("Couldn't parse command response: {}", err);
                panic!();
            }
        }
    }

    pub async fn delete(&self, client: &mut DiscordAPIClient) {
        if let Some(ref id) = self.get_id(client).await {
            match client
                .delete(&format!(
                    "applications/{}/commands/{}",
                    self.application_id, id
                ))
                .send()
                .await
            {
                Ok(response) => console_log!("Command {:?} deleted: {:#?}", self.name, response),
                Err(err) => console_error!("Command {:?} not deleted: {}", self.name, err),
            }
        } else {
            console_log!("Command not found!");
        }
    }
}

impl Default for ApplicationCommand {
    fn default() -> Self {
        ApplicationCommand {
            id: None,
            r#type: None,
            application_id: String::new(),
            guild_id: None,
            name: CommandName::Start,
            description: String::new(),
            options: None,
            default_member_permissions: None,
            dm_permission: Some(true),
            version: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationCommand {
    pub id: Option<String>,
    pub r#type: Option<CommandType>,
    pub application_id: String,
    pub guild_id: Option<String>,
    pub name: CommandName,
    pub description: String,
    pub options: Option<Vec<ApplicationCommandOption>>,
    pub default_member_permissions: Option<String>,
    pub dm_permission: Option<bool>,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationCommandOption {
    pub r#type: OptionType,
    pub name: String,
    pub description: String,
    pub required: Option<bool>,
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommandRegister {
    pub name: CommandName,
    pub description: String,
    pub options: Option<Vec<ApplicationCommandOption>>,
    pub default_member_permissions: Option<String>,
    pub dm_permission: Option<bool>,
    pub r#type: Option<CommandType>,
}

impl From<&ApplicationCommand> for CommandRegister {
    fn from(command: &ApplicationCommand) -> Self {
        Self {
            name: command.name,
            description: command.description.clone(),
            options: command.options.clone(),
            default_member_permissions: command.default_member_permissions.clone(),
            dm_permission: command.dm_permission,
            r#type: command.r#type.clone(),
        }
    }
}

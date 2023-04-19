use serde::{Deserialize, Serialize};
use worker::{console_debug, console_error, console_log};

use crate::interaction::{CommandName, CommandType, OptionType};
use crate::DiscordAPIClient;

impl ApplicationCommand {
    pub async fn get_id(&self, client: &mut DiscordAPIClient) -> Option<String> {
        let response = match client
            .get(&format!("applications/{}/commands", self.application_id))
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
        let commands = match commands {
            Ok(commands) => commands,
            Err(err) => {
                console_error!("Couldn't parse commands response: {}", err);
                panic!();
            }
        };

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
        if let Some(ref id) = self.id {
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
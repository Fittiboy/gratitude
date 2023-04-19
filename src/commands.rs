use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use worker::{console_debug, console_error, console_log};

use crate::interaction::{
    CommandName, CommandType, Interaction, InteractionData, InteractionResponse, OptionType,
};
use crate::DiscordAPIClient;

#[async_trait]
pub trait Command {
    async fn handle(&self, interaction: Interaction) -> InteractionResponse;
    async fn get_id(&self, client: &mut DiscordAPIClient) -> Option<u8>;
    async fn register(&self, client: &mut DiscordAPIClient) -> Self;
    async fn delete(&self, client: &mut DiscordAPIClient);
}

#[async_trait]
impl Command for ApplicationCommand {
    async fn handle(&self, interaction: Interaction) -> InteractionResponse {
        match interaction.data.as_ref().expect("Only pings have no data") {
            InteractionData::ApplicationCommandData(data) => match data.name {
                CommandName::Start => interaction.handle_command(),
                CommandName::Stop => interaction.handle_command(),
                CommandName::Entry => interaction.handle_command(),
            },
            _ => unreachable!("Type of data is known at this point"),
        }
    }
    async fn get_id(&self, client: &mut DiscordAPIClient) -> Option<u8> {
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
            Some(command) => command.id,
            None => None,
        }
    }

    async fn register(&self, client: &mut DiscordAPIClient) -> Self {
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
    async fn delete(&self, client: &mut DiscordAPIClient) {
        if let Some(id) = self.id {
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
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationCommand {
    id: Option<u8>,
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
pub struct ApplicationCommandOption {
    r#type: OptionType,
    name: String,
    description: String,
    required: Option<bool>,
    min_length: Option<u32>,
    max_length: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommandRegister {
    name: CommandName,
    description: String,
    options: Option<Vec<ApplicationCommandOption>>,
    default_member_permissions: Option<String>,
    dm_permission: Option<bool>,
    r#type: Option<CommandType>,
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

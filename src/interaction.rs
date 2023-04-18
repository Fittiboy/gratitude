use worker::{console_error, console_log};

use crate::discord_token;
use crate::error::Error;

mod data_types;
pub use data_types::*;

impl Modal {
    pub fn with_name(name: String) -> Self {
        Modal {
            custom_id: "grateful_modal".into(),
            title: format!("{}'s Gratitude Journal", name),
            components: vec![ActionRow::with_text_entry()],
        }
    }
}

impl TextInput {
    pub fn new() -> Self {
        TextInput {
            r#type: 4,
            custom_id: "grateful_input".into(),
            style: 2,
            label: "Express your gratitude for something!".into(),
            min_length: 5,
            max_length: 1000,
            placeholder: "Today, I am grateful for...".into(),
        }
    }
}

impl Message {
    pub fn from_entry(journal_entry: Option<String>) -> Self {
        let content = match journal_entry {
            Some(text) => Some(format!(
                "Here's something you were grateful for in the past:\n{}",
                text
            )),
            None => Some("Hi there, welcome to gratitude bot!".into()),
        };
        Message {
            id: None,
            channel_id: None,
            content,
            components: Some(vec![ActionRow::with_entry_button()]),
        }
    }
}

impl ActionRow {
    fn with_entry_button() -> Self {
        ActionRow {
            r#type: 1,
            components: vec![Component::Button(Button::entry())],
        }
    }

    fn with_text_entry() -> Self {
        ActionRow {
            r#type: 1,
            components: vec![Component::TextInput(TextInput::new())],
        }
    }
}

impl Button {
    fn entry() -> Self {
        Button {
            r#type: 2,
            style: 3,
            label: "What are you grateful for today?".into(),
            custom_id: "grateful_button".into(),
            disabled: Some(false),
        }
    }
}

impl Interaction {
    fn handle_ping(&self) -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::Pong,
            data: None,
        }
    }

    fn handle_button(&self) -> InteractionResponse {
        if let Some(InteractionData::ComponentInteractionData(button)) = &self.data {
            match button.custom_id {
                CustomId::GratefulButton => self.grateful_button(),
            }
        } else {
            console_error!("The message component is guaranteed to be a button in handle_button");
            unreachable!();
        }
    }

    fn grateful_button(&self) -> InteractionResponse {
        let name = self
            .user
            .clone()
            .expect("Only users can click buttons")
            .username;
        console_log!("Handling button!");
        InteractionResponse {
            r#type: InteractionResponseType::Modal,
            data: Some(InteractionResponseData::Modal(Modal::with_name(name))),
        }
    }

    async fn handle_modal(&self, token: String) -> InteractionResponse {
        let (message_id, mut payload) = self.id_and_payload();
        Self::disable_button(&mut payload.components);
        console_log!("Payload to disable button: {:#?}", payload);

        let client = reqwest::Client::new();
        if let Err(error) = client
            .patch(format!(
                "https://discord.com/api/channels/{}/messages/{}",
                self.channel_id.clone().unwrap(),
                message_id,
            ))
            .header(reqwest::header::AUTHORIZATION, token)
            .json(&payload)
            .send()
            .await
            .unwrap()
            .error_for_status()
        {
            console_log!("Error disabling button: {}", error);
        }

        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(Message {
                id: None,
                channel_id: None,
                content: Some("Neat, the interaction worked!".into()),
                components: Some(vec![]),
            })),
        }
    }

    fn id_and_payload(&self) -> (String, MessageEdit) {
        let message = self.message.as_ref().unwrap();
        let message_id = message.id.clone().unwrap();
        let payload = message
            .components
            .clone()
            .expect("Messages with a modal always have at least one component");
        (
            message_id,
            MessageEdit {
                components: payload,
            },
        )
    }

    fn disable_button(payload: &mut Vec<ActionRow>) {
        match payload.first_mut().unwrap().components.first_mut().unwrap() {
            Component::Button(Button { disabled, .. }) => *disabled = Some(true),
            _ => {}
        }
    }

    pub(crate) async fn perform(
        &self,
        ctx: &mut worker::RouteContext<()>,
    ) -> Result<InteractionResponse, Error> {
        match self.r#type {
            InteractionType::Ping => Ok(self.handle_ping()),
            InteractionType::MessageComponent => Ok(self.handle_button()),
            InteractionType::ModalSubmit => {
                let token = discord_token(&ctx.env).unwrap();
                Ok(self.handle_modal(token).await)
            }
        }
    }
}

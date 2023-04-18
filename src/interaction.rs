use worker::{console_error, console_log, Env};

use crate::error::Error;
use crate::DiscordAPIBuilder;

mod data_types;
pub use data_types::*;

impl Interaction {
    pub async fn perform(
        &self,
        ctx: &mut worker::RouteContext<()>,
    ) -> Result<InteractionResponse, Error> {
        match self.r#type {
            InteractionType::Ping => Ok(self.handle_ping()),
            InteractionType::MessageComponent => Ok(self.handle_component()),
            InteractionType::ModalSubmit => Ok(self.handle_modal(&ctx.env).await),
        }
    }

    fn handle_ping(&self) -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::Pong,
            data: None,
        }
    }

    fn handle_component(&self) -> InteractionResponse {
        let InteractionData::ComponentInteractionData(component) = &self
            .data
            .as_ref()
            .expect("Component data should always be part of the interaction") else {
                console_error!("handle_component should only ever receive component data");
                unreachable!();
            };
        match component.component_type {
            ComponentType::Button => self.handle_button(&component.custom_id),
            _ => unimplemented!(
                "There are currently not other component types in use in this context"
            ),
        }
    }

    fn handle_button(&self, custom_id: &CustomId) -> InteractionResponse {
        match custom_id {
            CustomId::GratefulButton => self.handle_grateful_button(),
            _ => unreachable!("All button IDs are covered!"),
        }
    }

    fn handle_grateful_button(&self) -> InteractionResponse {
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

    async fn handle_modal(&self, env: &Env) -> InteractionResponse {
        self.disable_button(env).await;

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

    async fn disable_button(&self, env: &Env) {
        let (message_id, mut payload) = self.id_and_payload();
        Self::prepare_button_disable_payload(&mut payload);
        console_log!("Payload to disable button: {:#?}", payload);

        self.submit_disable_button_request(message_id, env, payload)
            .await;
    }

    fn prepare_button_disable_payload(payload: &mut MessageEdit) {
        let components = &mut payload.components;
        match components
            .first_mut()
            .unwrap()
            .components
            .first_mut()
            .unwrap()
        {
            Component::Button(Button { disabled, .. }) => *disabled = Some(true),
            _ => {}
        }
    }

    async fn submit_disable_button_request(
        &self,
        message_id: String,
        env: &Env,
        payload: MessageEdit,
    ) {
        let channel_id = self.channel_id.clone().unwrap();
        let client = DiscordAPIBuilder::new(env)
            .patch(&format!("channels/{}/messages/{}", channel_id, message_id,));
        if let Err(error) = client
            .json(&payload)
            .send()
            .await
            .unwrap()
            .error_for_status()
        {
            console_error!("Error disabling button: {}", error);
        }
    }
}

impl Modal {
    pub fn with_name(name: String) -> Self {
        Modal {
            custom_id: CustomId::GratefulModal,
            title: format!("{}'s Gratitude Journal", name),
            components: vec![ActionRow::with_text_entry()],
        }
    }
}

impl TextInput {
    pub fn new() -> Self {
        TextInput {
            r#type: ComponentType::TextInput,
            custom_id: CustomId::GratefulInput,
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
            r#type: ComponentType::ActionRow,
            components: vec![Component::Button(Button::entry())],
        }
    }

    fn with_text_entry() -> Self {
        ActionRow {
            r#type: ComponentType::ActionRow,
            components: vec![Component::TextInput(TextInput::new())],
        }
    }
}

impl Button {
    fn entry() -> Self {
        Button {
            r#type: ComponentType::Button,
            style: 3,
            label: "What are you grateful for today?".into(),
            custom_id: CustomId::GratefulButton,
            disabled: Some(false),
        }
    }
}

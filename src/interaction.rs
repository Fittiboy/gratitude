use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use worker::console_log;

use crate::error::Error;

#[derive(Deserialize_repr, Serialize)]
#[repr(u8)]
enum InteractionType {
    Ping = 1,
    MessageComponent = 3,
    ModalSubmit = 5,
}

#[derive(Deserialize_repr, Serialize)]
#[repr(u8)]
enum ComponentType {
    Button = 2,
}

#[allow(dead_code)]
#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub(crate) enum InteractionResponseType {
    Pong = 1,
    ChannelMessageWithSource = 4,
    ACKWithSource = 5,
    Modal = 9,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum InteractionResponseData {
    Modal(Modal),
    Message(Message),
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ModalSubmitData {
    custom_id: String,
    components: Vec<ActionRow>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TextInputSubmit {
    r#type: u8,
    custom_id: String,
    value: String,
}

#[derive(Deserialize, Serialize)]
struct MessageComponentData {
    custom_id: String,
    component_type: ComponentType,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct User {
    id: String,
    username: String,
    discriminator: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Modal {
    custom_id: String,
    title: String,
    components: Vec<ActionRow>,
}

impl Modal {
    pub fn with_name(name: String) -> Self {
        Modal {
            custom_id: "grateful_modal".into(),
            title: format!("{}'s Gratitude Journal", name),
            components: vec![ActionRow::with_textinput()],
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TextInput {
    r#type: u8,
    custom_id: String,
    style: u8,
    label: String,
    max_length: u32,
    placeholder: String,
}

impl TextInput {
    pub fn new() -> Self {
        TextInput {
            r#type: 4,
            custom_id: "grateful_input".into(),
            style: 2,
            label: "Express your gratitude for something!".into(),
            max_length: 1000,
            placeholder: "Today, I am grateful for...".into(),
        }
    }
}

#[derive(Serialize)]
pub struct Message {
    content: Option<String>,
    components: Vec<ActionRow>,
}

impl Message {
    pub fn new(journal_entry: Option<String>) -> Self {
        let content = match journal_entry {
            Some(text) => Some(format!(
                "Here's something you were grateful for in the past:\n{}",
                text
            )),
            None => Some("Hi there, welcome to gratitude bot!".into()),
        };
        Message {
            content,
            components: vec![ActionRow::new()],
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
struct ActionRow {
    r#type: u8,
    components: Vec<Component>,
}

impl ActionRow {
    fn new() -> Self {
        ActionRow {
            r#type: 1,
            components: vec![Component::Button(Button::new())],
        }
    }

    fn with_textinput() -> Self {
        ActionRow {
            r#type: 1,
            components: vec![Component::TextInput(TextInput::new())],
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum Component {
    Button(Button),
    TextInput(TextInput),
    TextInputSubmit(TextInputSubmit),
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Button {
    r#type: u8,
    style: u8,
    label: String,
    custom_id: String,
    disabled: bool,
}

impl Button {
    fn new() -> Self {
        Button {
            r#type: 2,
            style: 3,
            label: "What are you grateful for today?".into(),
            custom_id: "grateful_button".into(),
            disabled: false,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum InteractionData {
    ComponentInteractionData(MessageComponentData),
    ModalInteractionData(ModalSubmitData),
}

#[derive(Deserialize, Serialize)]
pub struct Interaction {
    r#type: InteractionType,
    data: Option<InteractionData>,
    token: String,
    guild_id: Option<String>,
    channel_id: Option<String>,
    user: Option<User>,
}

#[derive(Serialize)]
pub struct InteractionResponse {
    pub(crate) r#type: InteractionResponseType,
    pub(crate) data: Option<InteractionResponseData>,
}

impl Interaction {
    fn handle_ping(&self) -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::Pong,
            data: None,
        }
    }

    fn handle_button(&self) -> InteractionResponse {
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

    fn handle_modal(&self) -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(Message {
                content: Some("Neat, the interaction worked!".into()),
                components: vec![],
            })),
        }
    }

    pub(crate) async fn perform(
        &self,
        _ctx: &mut worker::RouteContext<()>,
    ) -> Result<InteractionResponse, Error> {
        match self.r#type {
            InteractionType::Ping => Ok(self.handle_ping()),
            InteractionType::MessageComponent => Ok(self.handle_button()),
            InteractionType::ModalSubmit => Ok(self.handle_modal()),
        }
    }
}

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::error::Error;
use crate::Message;

#[derive(Deserialize_repr, Serialize)]
#[repr(u8)]
enum InteractionType {
    Ping = 1,
    MessageComponent = 3,
    ModalSubmit = 5,
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
pub(crate) enum InteractionResponseData {
    Modal(Modal),
    Message(Message),
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ModalSubmitData {
    custom_id: String,
    components: Vec<TextInput>,
}

#[derive(Deserialize, Serialize)]
struct MessageComponentData {
    name: String,
    component_type: u8,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct User {
    id: String,
    username: String,
    discriminator: String,
}

#[derive(Deserialize, Serialize)]
enum InteractionData {
    ComponentInteractionData(MessageComponentData),
    ModalInteractionData(ModalSubmitData),
}

#[derive(Deserialize, Serialize)]
pub struct Interaction {
    #[serde(rename = "type")]
    ty: InteractionType,
    data: Option<InteractionData>,
    token: String,
    guild_id: Option<String>,
    channel_id: Option<String>,
    user: Option<User>,
}

#[derive(Serialize)]
pub struct InteractionResponse {
    #[serde(rename = "type")]
    pub(crate) ty: InteractionResponseType,
    pub(crate) data: Option<InteractionResponseData>,
}

impl Interaction {
    fn handle_ping(&self) -> InteractionResponse {
        InteractionResponse {
            ty: InteractionResponseType::Pong,
            data: None,
        }
    }

    fn handle_button(&self) -> InteractionResponse {
        InteractionResponse {
            ty: InteractionResponseType::Modal,
            data: Some(InteractionResponseData::Modal(Modal::new())),
        }
    }

    fn handle_modal(&self) -> InteractionResponse {
        InteractionResponse {
            ty: InteractionResponseType::ChannelMessageWithSource,
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
        match self.ty {
            InteractionType::Ping => Ok(self.handle_ping()),
            InteractionType::MessageComponent => Ok(self.handle_button()),
            InteractionType::ModalSubmit => Ok(self.handle_modal()),
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Modal {
    custom_id: String,
    title: String,
    components: Vec<TextInput>,
}

impl Modal {
    fn new() -> Self {
        Modal {
            custom_id: "grateful_modal".into(),
            title: "What are you grateful for?".into(),
            components: vec![TextInput::new()],
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
struct TextInput {
    r#type: u8,
    custom_id: String,
    style: u8,
    label: String,
    max_length: u32,
    placeholder: String,
}

impl TextInput {
    fn new() -> Self {
        TextInput {
            r#type: 4,
            custom_id: "grateful_input".into(),
            style: 2,
            label: "What are you grateful for right now?".into(),
            max_length: 1000,
            placeholder: "Today, I am grateful for...".into(),
        }
    }
}

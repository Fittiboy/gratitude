use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::error::Error;

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

#[derive(Deserialize, Serialize, Clone)]
pub struct ModalSubmitData {
    custom_id: String,
    components: Vec<TextInput>,
}

#[derive(Deserialize, Serialize)]
struct ModalInteractionData {
    name: String,
    options: Option<Vec<ModalSubmitData>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct User {
    id: String,
    username: String,
    discriminator: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct Member {
    user: Option<User>,
    nick: Option<String>,
    permissions: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Interaction {
    #[serde(rename = "type")]
    ty: InteractionType,
    data: Option<ModalInteractionData>,
    token: String,
    guild_id: Option<String>,
    channel_id: Option<String>,
    user: Option<User>,
    member: Option<Member>,
}

#[derive(Serialize)]
pub struct InteractionResponse {
    #[serde(rename = "type")]
    pub(crate) ty: InteractionResponseType,
    pub(crate) data: Option<Modal>,
}

impl Interaction {
    pub(crate) fn handle_ping(&self) -> InteractionResponse {
        InteractionResponse {
            ty: InteractionResponseType::Pong,
            data: None,
        }
    }

    pub(crate) fn handle_modal(&self) -> InteractionResponse {
        InteractionResponse {
            ty: InteractionResponseType::Modal,
            data: Some(Modal::new()),
        }
    }

    pub(crate) async fn perform(
        &self,
        _ctx: &mut worker::RouteContext<()>,
    ) -> Result<InteractionResponse, Error> {
        match self.ty {
            InteractionType::Ping => Ok(self.handle_ping()),
            InteractionType::ModalSubmit => Ok(self.handle_modal()),
            _ => Err(Error::InvalidPayload("Not implemented".into())),
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

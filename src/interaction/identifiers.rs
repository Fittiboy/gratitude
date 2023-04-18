use serde::Serialize;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Deserialize_repr, Serialize)]
#[repr(u8)]
pub enum InteractionType {
    Ping = 1,
    MessageComponent = 3,
    ModalSubmit = 5,
}

#[derive(Deserialize_repr, Serialize)]
#[repr(u8)]
pub enum ComponentType {
    Button = 2,
}

#[allow(dead_code)]
#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum InteractionResponseType {
    Pong = 1,
    ChannelMessageWithSource = 4,
    ACKWithSource = 5,
    Modal = 9,
}

use serde::Deserialize;

use serde::Serialize;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Deserialize, Serialize)]
pub struct Interaction {
    pub r#type: InteractionType,
    pub data: Option<InteractionData>,
    pub token: String,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub message: Option<Message>,
    pub user: Option<User>,
}

#[derive(Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum InteractionType {
    Ping = 1,
    ApplicationCommand = 2,
    MessageComponent = 3,
    ModalSubmit = 5,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum InteractionData {
    ComponentInteractionData(MessageComponentData),
    ModalInteractionData(ModalSubmitData),
    ApplicationCommandData(ApplicationCommandData),
}

#[derive(Deserialize, Serialize)]
pub struct MessageComponentData {
    pub custom_id: CustomId,
    pub component_type: ComponentType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum CustomId {
    #[serde(rename = "grateful_button")]
    GratefulButton,
    #[serde(rename = "grateful_input")]
    GratefulInput,
    #[serde(rename = "grateful_modal")]
    GratefulModal,
}

#[derive(Debug, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum ComponentType {
    ActionRow = 1,
    Button = 2,
    TextInput = 4,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ModalSubmitData {
    pub custom_id: CustomId,
    pub components: Vec<ActionRow>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ActionRow {
    pub r#type: ComponentType,
    pub components: Vec<Component>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum Component {
    Button(Button),
    TextInput(TextInput),
    TextInputSubmit(TextInputSubmit),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Button {
    pub r#type: ComponentType,
    pub style: u8,
    pub label: String,
    pub custom_id: CustomId,
    pub disabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TextInput {
    pub r#type: ComponentType,
    pub custom_id: CustomId,
    pub style: u8,
    pub label: String,
    pub min_length: u32,
    pub max_length: u32,
    pub placeholder: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TextInputSubmit {
    pub r#type: ComponentType,
    pub custom_id: CustomId,
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApplicationCommandData {
    pub id: String,
    pub name: CommandName,
    pub r#type: CommandType,
    pub options: Option<Vec<OptionData>>,
    pub guild_id: Option<String>,
    pub target_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub enum CommandName {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "entry")]
    Entry,
}

#[derive(Debug, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum CommandType {
    ChatInput = 1,
    User = 2,
    Message = 3,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OptionData {
    pub name: String,
    pub r#type: OptionType,
    pub value: Option<OptionValue>,
    pub options: Option<Vec<OptionData>>,
    pub focues: bool,
}

#[derive(Debug, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum OptionType {
    SubCommand = 1,
    SubCommandGroup = 2,
    r#String = 3,
    Integer = 4,
    Boolean = 5,
    User = 6,
    Channel = 7,
    Role = 8,
    Mentionable = 9,
    Number = 10,
    Attachment = 11,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum OptionValue {
    r#String(String),
    Integer(u32),
    Float(f64),
    Bool(bool),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub discriminator: String,
}

#[derive(Serialize)]
pub struct InteractionResponse {
    pub r#type: InteractionResponseType,
    pub data: Option<InteractionResponseData>,
}

#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum InteractionResponseType {
    Pong = 1,
    ChannelMessageWithSource = 4,
    ACKWithSource = 5,
    Modal = 9,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum InteractionResponseData {
    Modal(Modal),
    Message(Message),
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Modal {
    pub custom_id: CustomId,
    pub title: String,
    pub components: Vec<ActionRow>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub id: Option<String>,
    pub channel_id: Option<String>,
    pub content: Option<String>,
    pub components: Option<Vec<ActionRow>>,
}

#[derive(Debug, Serialize)]
pub struct MessageEdit {
    pub components: Vec<ActionRow>,
}

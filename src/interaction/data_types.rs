use crate::error::Error;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub type PingInteraction = Interaction<PingData>;
pub type CommandInteraction = Interaction<ApplicationCommandData>;
pub type ButtonInteraction = SingleComponentInteraction<Button>;
pub type SingleComponentInteraction<C> =
    XXInteraction<ComponentIdentifier, SingleComponentResponse<C>>;
pub type SingleComponentResponse<C> = XXMessageResponse<[SingleComponentActionRow<C>; 1]>;
pub type SingleComponentActionRow<C> = XXActionRow<[C; 1]>;
pub type ComponentInteraction = Interaction<ComponentIdentifier>;
pub type SingleTextModalInteraction = SingleComponentModalInteraction<TextInput>;
pub type SingleComponentModalInteraction<C> =
    XXInteraction<SingleComponentModalSubmit<C>, SingleComponentResponse<C>>;
pub type SingleComponentModalSubmit<C> = XXModalSubmitData<SingleComponentActionRow<[C; 1]>>;
pub type ModalInteraction = Interaction<ModalSubmitData>;

#[derive(Debug, Deserialize, Serialize)]
pub struct InteractionIdentifier {
    pub r#type: InteractionType,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct XXInteraction<T, C> {
    pub data: T,
    pub token: String,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub message: C,
    pub member: Option<Member>,
    pub user: Option<User>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Interaction<T> {
    pub data: T,
    pub token: String,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub message: Option<MessageResponse>,
    pub member: Option<Member>,
    pub user: Option<User>,
}

#[derive(Debug, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum InteractionType {
    Ping = 1,
    ApplicationCommand = 2,
    MessageComponent = 3,
    ModalSubmit = 5,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PingData;

#[derive(Debug, Deserialize, Serialize)]
pub struct ComponentIdentifier {
    pub custom_id: ComponentId,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum ComponentId {
    #[serde(rename = "grateful_button")]
    GratefulButton,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum ModalId {
    #[serde(rename = "grateful_modal")]
    GratefulModal,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum TextInputId {
    #[serde(rename = "grateful_input")]
    GratefulInput,
}

#[derive(Debug, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum InteractionComponentType {
    Button = 2,
}

#[derive(Debug, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum InteractionModalSubmitType {
    TextInput = 4,
}

#[derive(Debug, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum ActionRowType {
    ActionRow = 1,
}

#[derive(Debug, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum ComponentType {
    ActionRow = 1,
    Button = 2,
    TextInput = 4,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct XXModalSubmitData<C> {
    pub custom_id: ModalId,
    pub components: C,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModalSubmitData {
    pub custom_id: ModalId,
    pub components: Vec<ActionRow>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct XXActionRow<C> {
    pub r#type: ActionRowType,
    pub components: C,
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
    pub custom_id: ComponentId,
    pub disabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TextInput {
    pub r#type: ComponentType,
    pub custom_id: TextInputId,
    pub style: u8,
    pub label: String,
    pub min_length: u32,
    pub max_length: u32,
    pub placeholder: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TextInputSubmit {
    pub r#type: ComponentType,
    pub custom_id: TextInputId,
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
    #[serde(rename = "help")]
    Help,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Member {
    pub user: Option<User>,
    pub nick: Option<String>,
    pub avatar: Option<String>,
    pub roles: Vec<String>,
    pub joined_at: String,
    pub premium: Option<String>,
    pub deaf: bool,
    pub mute: bool,
    pub flags: u8,
    pub pending: Option<bool>,
    pub permissions: Option<String>,
    pub communication_disabled_until: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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
    Modal(ModalResponse),
    Message(MessageResponse),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModalResponse {
    pub custom_id: ModalId,
    pub title: String,
    pub components: Vec<ActionRow>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct XXMessageResponse<C> {
    pub id: Option<String>,
    pub channel_id: Option<String>,
    pub content: Option<String>,
    pub flags: Option<u16>,
    pub components: C,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct MessageResponse {
    pub id: Option<String>,
    pub channel_id: Option<String>,
    pub content: Option<String>,
    pub flags: Option<u16>,
    pub components: Option<Vec<ActionRow>>,
}

#[derive(Debug, Serialize)]
pub struct MessageEditResponse {
    pub components: Vec<ActionRow>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Channel {
    pub id: String,
    pub r#type: ChannelType,
    pub guild_id: Option<String>,
}

#[derive(Debug, Serialize_repr, Deserialize_repr, Clone)]
#[repr(u8)]
pub enum ChannelType {
    GuildText = 0,
    Dm = 1,
    GuildVoice = 2,
    GroupDm = 3,
    GuildCategory = 4,
    GuildAnnouncement = 5,
    AnnouncementThread = 10,
    PublicThread = 11,
    PrivateThread = 12,
    GuildStageVoice = 13,
    GuildDirectory = 14,
    GuildForum = 15,
}

pub trait MarkDeserialize<'a>
where
    Self: Sized,
{
    fn from_str(string: &'a str) -> Result<Self, Error>;
}

impl<'a, T: Deserialize<'a>> MarkDeserialize<'a> for T {
    fn from_str(string: &'a str) -> Result<Self, Error> {
        serde_json::from_str::<Self>(string).map_err(Error::JsonFailed)
    }
}

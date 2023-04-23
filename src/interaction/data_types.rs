use crate::error::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

pub type PingInteraction = Interaction<PingData, NoMessage>;
pub type CommandInteraction = Interaction<ApplicationCommandData, NoComponentMessage>;
pub type ButtonInteraction = SingleComponentInteraction<Button>;
pub type SingleTextModalButtonInteraction = SingleTextModalComponentInteraction<Button>;
pub type InteractionIdentifier = Interaction<GenericData, GenericMessage>;
pub type ComponentIdentifier = Interaction<ComponentIdData, GenericMessage>;

pub type NoComponent = Option<()>;
pub type NoMessage = Option<()>;
pub type NoComponentMessage = Message<NoComponent>;
pub type SingleComponentInteraction<C> = Interaction<ComponentIdData, SingleComponentMessage<C>>;
pub type SingleTextModalComponentInteraction<C> =
    SingleComponentModalInteraction<TextInputSubmit, C>;
pub type GenericData = Option<Value>;
pub type GenericMessage = Option<Value>;

pub type SingleComponentMessage<C> = Message<[SingleComponentActionRow<C>; 1]>;
pub type SingleComponentModalInteraction<C, B> =
    Interaction<SingleComponentModalSubmit<C>, SingleComponentMessage<B>>;

pub type SingleComponentActionRow<C> = XXActionRow<[C; 1]>;
pub type SingleComponentModalSubmit<C> = ModalSubmitData<SingleComponentActionRow<[C; 1]>>;
pub type SingleButtonActionRow = SingleComponentActionRow<Button>;

pub type SingleButtonMessage = SingleComponentMessage<Button>;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Interaction<T, C> {
    pub r#type: InteractionType,
    pub data: T,
    pub token: String,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub message: C,
    pub member: Option<Member>,
    pub user: Option<User>,
}

#[derive(Debug, Default, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum InteractionType {
    #[default]
    Ping = 1,
    ApplicationCommand = 2,
    MessageComponent = 3,
    ModalSubmit = 5,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PingData;

#[derive(Debug, Deserialize, Serialize)]
pub struct ComponentIdData {
    pub custom_id: CustomId,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum CustomId {
    #[default]
    #[serde(rename = "grateful_button")]
    GratefulButton,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum ModalId {
    #[default]
    #[serde(rename = "grateful_modal")]
    GratefulModal,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum TextInputId {
    #[default]
    #[serde(rename = "grateful_input")]
    GratefulInput,
}

#[derive(Debug, Default, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum ActionRowType {
    #[default]
    ActionRow = 1,
}

#[derive(Debug, Default, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum InteractionComponentType {
    #[default]
    Button = 2,
}

#[derive(Debug, Default, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum ModalComponentType {
    #[default]
    TextInput = 4,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ModalSubmitData<C> {
    pub custom_id: ModalId,
    pub components: C,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct XXActionRow<C> {
    pub r#type: ActionRowType,
    pub components: C,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ActionRow {
    pub r#type: ActionRowType,
    pub components: Vec<Component>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum Component {
    Button(Button),
    TextInput(TextInput),
    TextInputSubmit(TextInputSubmit),
}

impl Default for Component {
    fn default() -> Self {
        Self::Button(Default::default())
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Button {
    pub r#type: InteractionComponentType,
    pub style: u8,
    pub label: String,
    pub custom_id: CustomId,
    pub disabled: Option<bool>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct TextInput {
    pub r#type: ModalComponentType,
    pub custom_id: TextInputId,
    pub style: u8,
    pub label: String,
    pub min_length: u32,
    pub max_length: u32,
    pub placeholder: String,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct TextInputSubmit {
    pub r#type: ModalComponentType,
    pub custom_id: TextInputId,
    pub value: String,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ApplicationCommandData {
    pub id: String,
    pub name: CommandName,
    pub r#type: CommandType,
    pub options: Option<Vec<OptionData>>,
    pub guild_id: Option<String>,
    pub target_id: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub enum CommandName {
    #[default]
    #[serde(rename = "help")]
    Help,
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "entry")]
    Entry,
}

#[derive(Debug, Default, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum CommandType {
    #[default]
    ChatInput = 1,
    User = 2,
    Message = 3,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct OptionData {
    pub name: String,
    pub r#type: OptionType,
    pub value: Option<OptionValue>,
    pub options: Option<Vec<OptionData>>,
}

#[derive(Debug, Default, Deserialize_repr, Serialize_repr, Clone)]
#[repr(u8)]
pub enum OptionType {
    SubCommand = 1,
    SubCommandGroup = 2,
    #[default]
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

impl Default for OptionValue {
    fn default() -> Self {
        Self::r#String(Default::default())
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
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

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub discriminator: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct InteractionResponse {
    pub r#type: InteractionResponseType,
    pub data: Option<InteractionResponseData>,
}

#[derive(Debug, Default, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum InteractionResponseType {
    #[default]
    Pong = 1,
    ChannelMessageWithSource = 4,
    ACKWithSource = 5,
    Modal = 9,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InteractionResponseData {
    Message(MessageResponse),
    Modal(ModalResponse),
}

impl Default for InteractionResponseData {
    fn default() -> Self {
        Self::Message(Default::default())
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ModalResponse {
    pub custom_id: ModalId,
    pub title: String,
    pub components: Vec<ActionRow>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Message<C> {
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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ComponentEdit<C> {
    pub components: C,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct MessageEditResponse {
    pub components: Vec<ActionRow>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Channel {
    pub id: String,
    pub r#type: ChannelType,
    pub guild_id: Option<String>,
}

#[derive(Debug, Default, Serialize_repr, Deserialize_repr, Clone)]
#[repr(u8)]
pub enum ChannelType {
    #[default]
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

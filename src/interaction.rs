use worker::{console_error, console_log, kv::KvStore, Env};

use crate::error::Error;
use crate::{discord_token, message, DiscordAPIClient};

pub mod data_types;
pub use data_types::*;

impl Interaction {
    pub async fn perform(
        &self,
        ctx: &mut worker::RouteContext<()>,
    ) -> Result<InteractionResponse, Error> {
        match self.r#type {
            InteractionType::Ping => Ok(self.handle_ping()),
            InteractionType::ApplicationCommand => {
                let mut client = DiscordAPIClient::new(discord_token(&ctx.env).unwrap());
                let users_kv = ctx
                    .env
                    .kv("grateful_users")
                    .expect("Worker should have access to grateful_users binding");
                Ok(self.handle_command(&mut client, users_kv).await)
            }
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

    pub async fn handle_command(
        &self,
        client: &mut DiscordAPIClient,
        kv: KvStore,
    ) -> InteractionResponse {
        let (user_id, mut channel_id) = match self.user.as_ref() {
            Some(User { id, .. }) => {
                let user_id = id.clone();
                let channel_id = self
                    .channel_id
                    .clone()
                    .expect("If user struct is there, channel_id will be the DM channel");
                (user_id, channel_id)
            }
            None => (
                self.member
                    .as_ref()
                    .unwrap()
                    .user
                    .as_ref()
                    .unwrap()
                    .id
                    .clone(),
                String::new(),
            ),
        };
        let mut channel_payload = std::collections::HashMap::new();
        if channel_id.is_empty() {
            channel_payload.insert("recipient_id", user_id.clone());
            let response = client
                .post("users/@me/channels")
                .json(&channel_payload)
                .send()
                .await;
            let channel = match response {
                Ok(response) => response.json::<Channel>().await,
                Err(err) => {
                    console_error!("Couldn't get DM channel: {}", err);
                    return InteractionResponse::error();
                }
            };
            match channel {
                Ok(channel) => {
                    channel_id = channel.id;
                }
                Err(err) => {
                    console_error!("Couldn't get DM channel: {}", err);
                    return InteractionResponse::error();
                }
            }
        };
        let users = match kv.get("users").json::<Vec<message::User>>().await {
            Ok(Some(users)) => users,
            Ok(None) => {
                console_error!("User list unexpectedly empty!");
                return InteractionResponse::error();
            }
            Err(err) => {
                console_error!("Couldn't get list of users: {}", err);
                return InteractionResponse::error();
            }
        };

        match self.data.as_ref().expect("only pings have no data") {
            InteractionData::ApplicationCommandData(data) => match data.name {
                CommandName::Start => {
                    self.handle_start(data, client, kv, user_id, channel_id, users)
                        .await
                }
                CommandName::Stop => {
                    self.handle_stop(data, client, kv, user_id, channel_id, users)
                        .await
                }
                CommandName::Entry => {
                    self.handle_entry(data, client, kv, user_id, channel_id, users)
                        .await
                }
            },
            _ => unreachable!("Commands are always commands (shocking, I know!)"),
        }
    }

    async fn handle_start(
        &self,
        data: &ApplicationCommandData,
        client: &mut DiscordAPIClient,
        kv: KvStore,
        user_id: String,
        channel_id: String,
        mut users: Vec<message::User>,
    ) -> InteractionResponse {
        console_log!("Handling start!");
        if users.iter().find(|user| user.uid == user_id).is_some() {
            return InteractionResponse::already_active();
        } else {
            users.push(message::User {
                uid: user_id.clone(),
                channel_id: channel_id.clone(),
            });
            if let Err(err) = kv.put("users", users).unwrap().execute().await {
                console_error!("Couldn't add user to list: {}", err);
                return InteractionResponse::error();
            }
        }

        let payload = Message::welcome();
        let client = client
            .post(&format!("channels/{}/messages", channel_id))
            .json(&payload);
        if let Err(error) = client.send().await.unwrap().error_for_status() {
            console_error!("Error sending message to user {}: {}", user_id, error);
            return InteractionResponse::dms_closed();
        }
        console_log!("New user: {:?}", data.target_id);

        InteractionResponse::success()
    }

    async fn handle_stop(
        &self,
        data: &ApplicationCommandData,
        client: &mut DiscordAPIClient,
        kv: KvStore,
        user_id: String,
        channel_id: String,
        mut users: Vec<message::User>,
    ) -> InteractionResponse {
        console_log!("Handling stop!");
        if users.iter().find(|user| user.uid == user_id).is_none() {
            return InteractionResponse::not_active();
        } else {
            let original_length = users.len();
            users.retain(|user| user.uid != user_id);
            let length_after = users.len();
            if !(original_length - 1 == length_after) {
                console_error!(
                    "Length after removing not one less. Old: {}, New: {}",
                    original_length,
                    length_after
                );
                return InteractionResponse::error();
            } else {
                if let Err(err) = kv.put("users", users).unwrap().execute().await {
                    console_error!("Couldn't remove user from list: {}", err);
                    return InteractionResponse::error();
                }
            }
        }

        let payload = Message::goodbye();
        let client = client
            .post(&format!("channels/{}/messages", channel_id))
            .json(&payload);
        if let Err(error) = client.send().await.unwrap().error_for_status() {
            console_error!("Error sending message to user {}: {}", user_id, error);
            return InteractionResponse::dms_closed();
        }
        console_log!("User removed: {:?}", data.target_id);

        InteractionResponse::success()
    }

    async fn handle_entry(
        &self,
        data: &ApplicationCommandData,
        client: &mut DiscordAPIClient,
        kv: KvStore,
        user_id: String,
        channel_id: String,
        mut users: Vec<message::User>,
    ) -> InteractionResponse {
        InteractionResponse::success()
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
        let entry = self.entry();
        self.add_entry(env, &entry).await;
        let token = discord_token(env).unwrap();
        self.disable_button(token).await;

        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(Message {
                id: None,
                channel_id: None,
                content: Some(format!("You said: {}", entry)),
                flags: None,
                components: Some(vec![]),
            })),
        }
    }

    fn entry(&self) -> String {
        let action_row = self.modal_action_row();
        let action_row = action_row.components.iter().next().unwrap();
        let Component::TextInputSubmit(TextInputSubmit { value, .. }) =
            action_row.components.iter().next().unwrap() else {
                unreachable!("Modals support only text inputs");
            };
        value.to_owned()
    }

    async fn add_entry(&self, env: &Env, entry: &str) {
        let id = &self
            .user
            .clone()
            .expect("only users can interact with modals")
            .id;
        let kv = env
            .kv("thankful")
            .expect("worker should have binding to thankful namespace");
        let mut entries = self.get_entries(&kv, &id).await;
        entries.push(entry.to_string());
        kv.put(id, entries)
            .unwrap()
            .execute()
            .await
            .expect("should be able to serialize entries");
    }

    async fn get_entries(&self, kv: &KvStore, id: &String) -> Vec<String> {
        match kv.get(id).text().await {
            Ok(Some(text)) => serde_json::from_str(&text).unwrap(),
            Ok(None) => Vec::new(),
            Err(err) => {
                console_error!("Couldn't get entries: {}", err);
                panic!();
            }
        }
    }

    fn modal_action_row(&self) -> ModalSubmitData {
        match self
            .data
            .as_ref()
            .expect("Modal interactions always have data")
        {
            InteractionData::ModalInteractionData(ref data) => data.clone(),
            _ => unreachable!("Modal type is guaranteed at this point"),
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

    async fn disable_button(&self, token: String) {
        let (message_id, mut payload) = self.id_and_payload();
        Self::prepare_button_disable_payload(&mut payload);
        console_log!("Payload to disable button: {:#?}", payload);

        self.submit_disable_button_request(message_id, token, payload)
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
        token: String,
        payload: MessageEdit,
    ) {
        let channel_id = self.channel_id.clone().unwrap();
        let client = DiscordAPIClient::new(token)
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

impl InteractionResponse {
    fn success() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(Message::success())),
        }
    }

    fn error() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(Message::error())),
        }
    }

    fn dms_closed() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(Message::dms_closed())),
        }
    }

    fn already_active() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(Message::already_active())),
        }
    }

    fn not_active() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(Message::not_active())),
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
            placeholder:
                "Today, I am grateful for… (a nice meal, someone smiling at me, how I perfectly parked my car)"
                    .to_string(),
        }
    }
}

impl Message {
    pub fn welcome() -> Self {
        let content = Some("Hi there! welcome to Gratitude Bot! 🥳".into());
        Message {
            content,
            components: Some(vec![ActionRow::with_entry_button()]),
            ..Default::default()
        }
    }

    pub fn goodbye() -> Self {
        let content = Some("You will no longer receive reminders! See you around! 😊".into());
        Message {
            content,
            components: Some(vec![ActionRow::with_entry_button()]),
            ..Default::default()
        }
    }

    pub fn from_entry(journal_entry: Option<String>) -> Self {
        let content = match journal_entry {
            Some(text) => Some(format!(
                "**Here's something you were grateful for in the past:**\n{}",
                text
            )),
            None => Some("Hope you're having a great day!".into()),
        };
        Message {
            content,
            components: Some(vec![ActionRow::with_entry_button()]),
            ..Default::default()
        }
    }

    pub fn success() -> Self {
        Message {
            content: Some(
                "It lookes like that worked! If it didn't do what you expected, contact Fitti#6969"
                    .into(),
            ),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn error() -> Self {
        Message {
            content: Some(
                "Oh no! It looks like something went wrong!\nAsk Fitti#6969 for help!".into(),
            ),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn dms_closed() -> Self {
        Message {
            content: Some(format!(
                "It looks like the bot can't DM you! Check your privacy settings: {}",
                "https://support.discord.com/hc/en-us/articles/217916488-Blocking-Privacy-Settings",
            )),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn already_active() -> Self {
        Message {
            content: Some(format!(
                "Looks like you're already an active user! {} {}",
                "The bot will randomly send you reminders about once per day.",
                "Use the /stop command to stop receiving those reminders!"
            )),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn not_active() -> Self {
        Message {
            content: Some(format!(
                "Looks like you're not an active user! {} {}",
                "The bot will not send you reminders.",
                "Use the /start command to start receiving those reminders!"
            )),
            flags: Some(1 << 6),
            ..Default::default()
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

impl Default for Message {
    fn default() -> Self {
        Message {
            id: None,
            channel_id: None,
            content: None,
            flags: None,
            components: None,
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

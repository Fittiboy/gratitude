use serde_json::{from_str, to_string};
use worker::{console_error, console_log, kv::KvStore, Env};

use crate::error::Error;
use crate::{discord_token, users::BotUser, DiscordAPIClient};

pub mod data_types;
pub use data_types::*;

impl Interaction {
    pub async fn perform(
        &self,
        ctx: &mut worker::RouteContext<()>,
    ) -> Result<InteractionResponse, Error> {
        let thankful_kv = ctx
            .env
            .kv("thankful")
            .expect("Worker should have access to thankful binding");
        match self.r#type {
            InteractionType::Ping => Ok(self.handle_ping()),
            InteractionType::ApplicationCommand => {
                let mut client = DiscordAPIClient::new(discord_token(&ctx.env).unwrap());
                let users_kv = ctx
                    .env
                    .kv("grateful_users")
                    .expect("Worker should have access to grateful_users binding");
                Ok(self
                    .handle_command(&mut client, users_kv, thankful_kv)
                    .await)
            }
            InteractionType::MessageComponent => Ok(self.handle_component()),
            InteractionType::ModalSubmit => Ok(self.handle_modal(&ctx.env, thankful_kv).await),
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
        users_kv: KvStore,
        thankful_kv: KvStore,
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
        let users = match users_kv.get("users").json::<Vec<BotUser>>().await {
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
        let user = BotUser {
            uid: user_id.clone(),
            channel_id: channel_id.clone(),
        };
        let add_key = format!("ADD {}", to_string(&user).unwrap());
        let delete_key = format!("DELETE {}", user_id);

        match self.data.as_ref().expect("only pings have no data") {
            InteractionData::ApplicationCommand(data) => match data.name {
                CommandName::Start => {
                    self.handle_start(
                        client, users_kv, user_id, channel_id, add_key, delete_key, users,
                    )
                    .await
                }
                CommandName::Stop => {
                    self.handle_stop(
                        client, users_kv, user_id, channel_id, add_key, delete_key, users,
                    )
                    .await
                }
                CommandName::Entry => {
                    self.handle_entry(client, channel_id, user_id, thankful_kv)
                        .await
                }
            },
            _ => unreachable!("Commands are always commands (shocking, I know!)"),
        }
    }

    async fn handle_start(
        &self,
        client: &mut DiscordAPIClient,
        kv: KvStore,
        user_id: String,
        channel_id: String,
        add_key: String,
        delete_key: String,
        users: Vec<BotUser>,
    ) -> InteractionResponse {
        console_log!("Handling start!");
        if kv.get(&delete_key).text().await.unwrap().is_some() {
            if let Err(err) = kv.delete(&delete_key).await {
                console_error!("Couldn't remove delete key from kv: {}", err);
                return InteractionResponse::error();
            }
        } else if users.iter().any(|user| user.uid == user_id)
            || kv.get(&add_key).text().await.unwrap().is_some()
        {
            return InteractionResponse::already_active();
        } else if let Err(err) = kv.put(&add_key, "FOOP").unwrap().execute().await {
            console_error!("Couldn't add user to list: {}", err);
            return InteractionResponse::error();
        }
        let payload = Message::welcome();
        let client = client
            .post(&format!("channels/{}/messages", channel_id))
            .json(&payload);
        if let Err(error) = client.send().await.unwrap().error_for_status() {
            console_error!("Error sending message to user {}: {}", user_id, error);
            return InteractionResponse::dms_closed();
        }
        console_log!("New user: {:?}", user_id);

        InteractionResponse::success()
    }

    async fn handle_stop(
        &self,
        client: &mut DiscordAPIClient,
        kv: KvStore,
        user_id: String,
        channel_id: String,
        add_key: String,
        delete_key: String,
        mut users: Vec<BotUser>,
    ) -> InteractionResponse {
        console_log!("Handling stop!");
        if kv.get(&add_key).text().await.unwrap().is_some() {
            if let Err(err) = kv.delete(&add_key).await {
                console_error!("Couldn't remove delete key from kv: {}", err);
                return InteractionResponse::error();
            }
        } else if !users.iter().any(|user| user.uid == user_id)
            || kv.get(&delete_key).text().await.unwrap().is_some()
        {
            return InteractionResponse::not_active();
        } else {
            let original_length = users.len();
            users.retain(|user| user.uid != user_id);
            let length_after = users.len();
            if original_length - 1 != length_after {
                console_error!(
                    "Length after removing not one less. Old: {}, New: {}",
                    original_length,
                    length_after
                );
                return InteractionResponse::error();
            } else if let Err(err) = kv.put(&delete_key, "POOF").unwrap().execute().await {
                console_error!("Couldn't remove user from list: {}", err);
                return InteractionResponse::error();
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
        console_log!("User removed: {:?}", user_id);

        InteractionResponse::success()
    }

    async fn handle_entry(
        &self,
        client: &mut DiscordAPIClient,
        channel_id: String,
        user_id: String,
        thankful_kv: KvStore,
    ) -> InteractionResponse {
        console_log!("Handling entry");
        let entry = self.entry();
        self.add_entry(thankful_kv, &entry).await;
        let payload = Message {
            id: None,
            channel_id: None,
            content: Some(format!("**You added the following entry:**\n{}", entry)),
            flags: None,
            components: Some(vec![]),
        };
        let client = client
            .post(&format!("channels/{}/messages", channel_id))
            .json(&payload);
        if let Err(error) = client.send().await.unwrap().error_for_status() {
            console_error!("Error sending message to user {}: {}", user_id, error);
            return InteractionResponse::dms_closed();
        }

        InteractionResponse::success()
    }

    fn handle_component(&self) -> InteractionResponse {
        let InteractionData::Component(component) = &self
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

    async fn handle_modal(&self, env: &Env, thankful_kv: KvStore) -> InteractionResponse {
        let entry = self.entry();
        self.add_entry(thankful_kv, &entry).await;
        let token = discord_token(env).unwrap();
        self.disable_button(token).await;

        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(Message {
                id: None,
                channel_id: None,
                content: Some(format!("**You added the following entry:**\n{}", entry)),
                flags: None,
                components: Some(vec![]),
            })),
        }
    }

    fn entry(&self) -> String {
        match self.r#type {
            InteractionType::ModalSubmit => {
                let action_row = self.modal_action_row();
                let action_row = action_row.components.first().unwrap();
                let Component::TextInputSubmit(TextInputSubmit { value, .. }) =
                    action_row.components.first().unwrap() else {
                        unreachable!("Modals support only text inputs");
                    };
                value.to_owned()
            }
            InteractionType::ApplicationCommand => match self.data.as_ref().unwrap() {
                InteractionData::ApplicationCommand(data) => {
                    let OptionData { value, .. } = data.options.as_ref().unwrap().first().unwrap();
                    let OptionValue::String(ref value) = value.as_ref().unwrap() else { unreachable!("Value guaranteed by Discord") };
                    value.to_owned()
                }
                _ => unreachable!("We know it's a command now!"),
            },
            _ => unreachable!("No entries anywhere else!"),
        }
    }

    async fn add_entry(&self, thankful_kv: KvStore, entry: &str) {
        let id = match self.user.as_ref() {
            Some(User { id, .. }) => id,
            None => match self.member {
                Some(Member { ref user, .. }) => &user.as_ref().unwrap().id,
                None => unreachable!("There should always be a member or a user!"),
            },
        };
        let mut entries = self.get_entries(&thankful_kv, id).await;
        entries.push(entry.to_string());
        thankful_kv
            .put(id, entries)
            .unwrap()
            .execute()
            .await
            .expect("should be able to serialize entries");
    }

    async fn get_entries(&self, kv: &KvStore, id: &str) -> Vec<String> {
        match kv.get(id).text().await {
            Ok(Some(text)) => from_str(&text).unwrap(),
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
            InteractionData::Modal(ref data) => data.clone(),
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
        if let Component::Button(Button { disabled, .. }) = components
            .first_mut()
            .unwrap()
            .components
            .first_mut()
            .unwrap()
        {
            *disabled = Some(true)
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
                "It looks like that worked! If it didn't do what you expected, contact Fitti#6969"
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

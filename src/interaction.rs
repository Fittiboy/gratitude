use serde_json::{from_str, to_string};
use worker::{console_error, console_log, kv::KvStore};

use crate::discord;
use crate::users::BotUser;

pub mod data_types;
pub use data_types::*;

impl Interaction<PingData> {
    pub async fn handle(&self) -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::Pong,
            data: None,
        }
    }
}

impl Interaction<ApplicationCommandData> {
    pub async fn handle(
        &self,
        client: &mut discord::Client,
        users_kv: KvStore,
        thankful_kv: KvStore,
    ) -> InteractionResponse {
        let (user_id, mut channel_id) = self.ids();
        if channel_id.is_empty() {
            channel_id = match self.dm_channel(&user_id, client).await {
                Some(id) => id,
                None => return InteractionResponse::error(),
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

        match self.data.name {
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
            CommandName::Help => InteractionResponse::help(),
        }
    }

    pub fn ids(&self) -> (String, String) {
        match self.user.as_ref() {
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
        }
    }

    pub async fn dm_channel(&self, user_id: &str, client: &mut discord::Client) -> Option<String> {
        let mut channel_payload = std::collections::HashMap::new();
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
                return None;
            }
        };
        match channel {
            Ok(channel) => {
                return Some(channel.id);
            }
            Err(err) => {
                console_error!("Couldn't get DM channel: {}", err);
                return None;
            }
        }
    }

    async fn handle_start(
        &self,
        client: &mut discord::Client,
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
        let payload = MessageResponse::welcome();
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
        client: &mut discord::Client,
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

        let payload = MessageResponse::goodbye();
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
        client: &mut discord::Client,
        channel_id: String,
        user_id: String,
        thankful_kv: KvStore,
    ) -> InteractionResponse {
        console_log!("Handling entry");
        let entry = self.entry();
        self.add_entry(thankful_kv, &entry).await;
        let payload = MessageResponse {
            id: None,
            channel_id: None,
            content: Some(format!("__**You added the following entry:**__\n{}", entry)),
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

    fn entry(&self) -> String {
        let OptionData { value, .. } = self.data.options.as_ref().unwrap().first().unwrap();
        let OptionValue::String(ref value) = value.as_ref().unwrap() else { unreachable!("Value guaranteed by Discord") };
        value.to_owned()
    }
}

impl ButtonInteraction {
    pub fn handle_grateful(&self) -> InteractionResponse {
        let name = self
            .user
            .clone()
            .expect("Only users can click buttons")
            .username;
        console_log!("Handling button!");
        InteractionResponse {
            r#type: InteractionResponseType::Modal,
            data: Some(InteractionResponseData::Modal(ModalResponse::with_name(
                name,
            ))),
        }
    }
}

impl Interaction<ModalSubmitData> {
    pub async fn handle(
        &self,
        thankful_kv: KvStore,
        client: &mut discord::Client,
    ) -> InteractionResponse {
        let entry = self.entry();
        self.add_entry(thankful_kv, &entry).await;
        self.disable_button(client).await;

        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(MessageResponse {
                id: None,
                channel_id: None,
                content: Some(format!("**You added the following entry:**\n{}", entry)),
                flags: None,
                components: Some(vec![]),
            })),
        }
    }

    fn entry(&self) -> String {
        let action_row = self.data.components.first().unwrap();
        let Component::TextInputSubmit(TextInputSubmit { value, .. }) =
            action_row.components.first().unwrap() else {
                unreachable!("Modals support only text inputs");
            };
        value.to_owned()
    }

    async fn disable_button(&self, client: &mut discord::Client) {
        let (message_id, mut payload) = self.id_and_payload();
        Self::prepare_button_disable_payload(&mut payload);
        console_log!("Payload to disable button: {:#?}", payload);

        self.submit_disable_button_request(message_id, client, payload)
            .await;
    }

    fn id_and_payload(&self) -> (String, MessageEditResponse) {
        let message = self.message.as_ref().unwrap();
        let message_id = message.id.clone().unwrap();
        let payload = message
            .components
            .clone()
            .expect("Messages with a modal always have at least one component");
        (
            message_id,
            MessageEditResponse {
                components: payload,
            },
        )
    }

    fn prepare_button_disable_payload(payload: &mut MessageEditResponse) {
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
        client: &mut discord::Client,
        payload: MessageEditResponse,
    ) {
        let channel_id = self.channel_id.clone().unwrap();
        if let Err(error) = client
            .patch(&format!("channels/{}/messages/{}", channel_id, message_id,))
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

impl<T> Interaction<T> {
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
}

impl InteractionResponse {
    #[allow(dead_code)]
    fn not_implemented() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(
                MessageResponse::not_implemented(),
            )),
        }
    }

    fn help() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(MessageResponse::help())),
        }
    }

    fn success() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(MessageResponse::success())),
        }
    }

    fn error() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(MessageResponse::error())),
        }
    }

    fn dms_closed() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(
                MessageResponse::dms_closed(),
            )),
        }
    }

    fn already_active() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(
                MessageResponse::already_active(),
            )),
        }
    }

    fn not_active() -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(
                MessageResponse::not_active(),
            )),
        }
    }
}

impl ModalResponse {
    pub fn with_name(name: String) -> Self {
        ModalResponse {
            custom_id: ModalId::GratefulModal,
            title: format!("{}'s Gratitude Journal", name),
            components: vec![ActionRow::with_text_entry()],
        }
    }
}

impl TextInput {
    pub fn new() -> Self {
        TextInput {
            r#type: ComponentType::TextInput,
            custom_id: TextInputId::GratefulInput,
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

impl MessageResponse {
    pub fn not_implemented() -> Self {
        MessageResponse {
            content: Some("This command is not yet implemented! Coming soon!".into()),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn help() -> Self {
        MessageResponse {
            content: Some(
                concat!(
                    "__**Welcome to Gratitude Bot!**__\n",
                    "*This bot makes you focus on the positive things in life!*\n\n",
                    "It does this by randomly, once every few days on average, nudging ",
                    "you to add an entry to the gratitude journal it keeps for you, while ",
                    "reminding you of things you said you were grateful for in the past. ",
                    "Anything goes here: The smallest thing that made you smile today, or ",
                    "that big event that changed your life last month. Over time, your ",
                    "brain will change to be more aware of the nice things in life, help ",
                    "you appreciate what you have right now!\n\nYou can use **/start** to ",
                    "sign up for those reminders, **/stop** to stop receiving them, and ",
                    "**/entry** to add something to the journal at any point!\n\n",
                    "*The bot is open source, and you can view (and copy!) the code ",
                    "right here: <https://github.com/Fittiboy/gratitude>!*"
                )
                .to_string(),
            ),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn welcome() -> Self {
        let content = Some(
            concat!(
                "**Hi there! Thank you for deciding to use Gratitude Bot! 🥳**\n",
                "The bot will send you reminders, every few days or so, to ",
                "think about someting you are grateful for, and ask you to add it ",
                "to your journal! You can use /stop at any time to stop these reminders",
                "\n\n👇 Click the button below to make an entry into your journal right now!"
            )
            .to_string(),
        );
        MessageResponse {
            content,
            components: Some(vec![ActionRow::with_entry_button()]),
            ..Default::default()
        }
    }

    pub fn goodbye() -> Self {
        let content = Some(
            concat!(
                "**You will no longer receive reminders! See you around! 😊**\n",
                "Rememeber that you can still use **/entry** to make entries, ",
                "and **/start** to receive these reminders again!"
            )
            .into(),
        );
        MessageResponse {
            content,
            ..Default::default()
        }
    }

    pub fn from_entry(journal_entry: Option<String>) -> Self {
        let content = match journal_entry {
            Some(text) => Some(format!(
                "__**Here's something you said you were grateful for in the past:**__\n{}",
                text
            )),
            None => Some("Hope you're having a great day!".into()),
        };
        MessageResponse {
            content,
            components: Some(vec![ActionRow::with_entry_button()]),
            ..Default::default()
        }
    }

    pub fn success() -> Self {
        MessageResponse {
            content: Some(
                "**It looks like that worked! 🥳** If it didn't do what you expected, contact Fitti#6969"
                    .to_string(),
            ),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn error() -> Self {
        MessageResponse {
            content: Some(
                "Oh no! It looks like something went wrong!\nAsk Fitti#6969 for help!".into(),
            ),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn dms_closed() -> Self {
        MessageResponse {
            content: Some(format!(
                "It looks like the bot can't DM you! Check your privacy settings: {}",
                "https://support.discord.com/hc/en-us/articles/217916488-Blocking-Privacy-Settings",
            )),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn already_active() -> Self {
        MessageResponse {
            content: Some(
                concat!(
                    "Looks like you're already an active user! ",
                    "The bot will randomly send you reminders every few days.\n",
                    "Use the **/stop** command to stop receiving those reminders! ",
                    "Remember that you can use **/entry** to add something to your ",
                    "journal at any time!"
                )
                .to_string(),
            ),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn not_active() -> Self {
        MessageResponse {
            content: Some(
                concat!(
                    "Looks like you're not an active user! ",
                    "The bot will not send you reminders.\n",
                    "Use the **/start** command to start receiving those reminders! ",
                    "Remember that you can always use **/entry** to add something to ",
                    "your journal!"
                )
                .to_string(),
            ),
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
            custom_id: ComponentId::GratefulButton,
            disabled: Some(false),
        }
    }
}

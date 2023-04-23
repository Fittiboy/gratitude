use serde_json::{from_str, to_string};
use worker::{console_error, console_log, kv::KvStore};

use crate::discord;
use crate::users::BotUser;

pub mod data_types;
pub use data_types::*;

impl PingInteraction {
    pub async fn handle(&self) -> InteractionResponse<NoResponseData> {
        InteractionResponse {
            r#type: InteractionResponseType::Pong,
            data: None,
        }
    }
}

impl CommandInteraction {
    pub async fn handle(
        &self,
        client: &mut discord::Client,
        users_kv: KvStore,
        thankful_kv: KvStore,
    ) -> SimpleMessageResponse {
        let (user_id, mut channel_id) = self.ids();
        if channel_id.is_empty() {
            channel_id = match self.dm_channel(&user_id, client).await {
                Some(id) => id,
                None => return SimpleMessageResponse::error(),
            }
        };
        let users = match users_kv.get("users").json::<Vec<BotUser>>().await {
            Ok(Some(users)) => users,
            Ok(None) => {
                console_error!("User list unexpectedly empty!");
                return SimpleMessageResponse::error();
            }
            Err(err) => {
                console_error!("Couldn't get list of users: {}", err);
                return SimpleMessageResponse::error();
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
            CommandName::Help => SimpleMessageResponse::help(),
        }
    }

    pub fn ids(&self) -> (String, String) {
        match self.user.as_ref() {
            Some(User { id, .. }) => {
                let user_id = id.clone();
                let channel_id = self
                    .channel_id
                    .clone()
                    //TODO: Make illegal state unrepresentable
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
    ) -> SimpleMessageResponse {
        console_log!("Handling start!");
        if kv.get(&delete_key).text().await.unwrap().is_some() {
            if let Err(err) = kv.delete(&delete_key).await {
                console_error!("Couldn't remove delete key from kv: {}", err);
                return SimpleMessageResponse::error();
            }
        } else if users.iter().any(|user| user.uid == user_id)
            || kv.get(&add_key).text().await.unwrap().is_some()
        {
            return SimpleMessageResponse::already_active();
        } else if let Err(err) = kv.put(&add_key, "FOOP").unwrap().execute().await {
            console_error!("Couldn't add user to list: {}", err);
            return SimpleMessageResponse::error();
        }
        let payload = Message::welcome();
        let client = client
            .post(&format!("channels/{}/messages", channel_id))
            .json(&payload);
        if let Err(error) = client.send().await.unwrap().error_for_status() {
            console_error!("Error sending message to user {}: {}", user_id, error);
            return SimpleMessageResponse::dms_closed();
        }
        console_log!("New user: {:?}", user_id);

        SimpleMessageResponse::success()
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
    ) -> SimpleMessageResponse {
        console_log!("Handling stop!");
        if kv.get(&add_key).text().await.unwrap().is_some() {
            if let Err(err) = kv.delete(&add_key).await {
                console_error!("Couldn't remove delete key from kv: {}", err);
                return SimpleMessageResponse::error();
            }
        } else if !users.iter().any(|user| user.uid == user_id)
            || kv.get(&delete_key).text().await.unwrap().is_some()
        {
            return SimpleMessageResponse::not_active();
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
                return SimpleMessageResponse::error();
            } else if let Err(err) = kv.put(&delete_key, "POOF").unwrap().execute().await {
                console_error!("Couldn't remove user from list: {}", err);
                return SimpleMessageResponse::error();
            }
        }

        let payload = NoComponentMessage::goodbye();
        let client = client
            .post(&format!("channels/{}/messages", channel_id))
            .json(&payload);
        if let Err(error) = client.send().await.unwrap().error_for_status() {
            console_error!("Error sending message to user {}: {}", user_id, error);
            return SimpleMessageResponse::dms_closed();
        }
        console_log!("User removed: {:?}", user_id);

        SimpleMessageResponse::success()
    }

    async fn handle_entry(
        &self,
        client: &mut discord::Client,
        channel_id: String,
        user_id: String,
        thankful_kv: KvStore,
    ) -> SimpleMessageResponse {
        console_log!("Handling entry");
        let entry = self.entry();
        self.add_entry(thankful_kv, &entry).await;
        let payload = NoComponentMessage {
            content: Some(format!("__**You added the following entry:**__\n{}", entry)),
            ..Default::default()
        };
        let client = client
            .post(&format!("channels/{}/messages", channel_id))
            .json(&payload);
        if let Err(error) = client.send().await.unwrap().error_for_status() {
            console_error!("Error sending message to user {}: {}", user_id, error);
            return SimpleMessageResponse::dms_closed();
        }

        SimpleMessageResponse::success()
    }

    fn entry(&self) -> String {
        let OptionData { value, .. } = self.data.options.as_ref().unwrap().first().unwrap();
        let OptionValue::String(ref value) = value.as_ref().unwrap() else { unreachable!("Value guaranteed by Discord") };
        value.to_owned()
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
}

impl ButtonInteraction {
    pub fn handle_grateful(&self) -> SingleTextInputModalResponse {
        let name = self
            .user
            .clone()
            //TODO: Make illegal state unrepresentable
            .expect("Only users can click buttons")
            .username;
        console_log!("Handling button!");
        SingleTextInputModalResponse {
            r#type: InteractionResponseType::Modal,
            data: ModalResponse::with_name(name),
        }
    }
}

impl SingleTextModalButtonInteraction {
    pub async fn handle(
        &mut self,
        thankful_kv: KvStore,
        client: &mut discord::Client,
    ) -> SimpleMessageResponse {
        self.add_entry(thankful_kv).await;
        self.disable_button(client).await;

        SimpleMessageResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: NoComponentMessage {
                content: Some(format!(
                    "**You added the following entry:**\n{}",
                    self.entry()
                )),
                ..Default::default()
            },
        }
    }

    async fn add_entry(&self, thankful_kv: KvStore) {
        let entry = self.entry();
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

    fn entry(&self) -> &str {
        &self.data.components[0].components[0].value
    }

    async fn disable_button(&mut self, client: &mut discord::Client) {
        self.message.components[0].components[0].disabled = Some(true);
        self.submit_disable_button_request(client).await;
    }

    async fn submit_disable_button_request(&self, client: &mut discord::Client) {
        if let Err(error) = client
            .patch(&format!(
                "channels/{}/messages/{}",
                self.channel_id.as_ref().unwrap(),
                self.message.id.as_ref().unwrap()
            ))
            .json(&self.message)
            .send()
            .await
            .unwrap()
            .error_for_status()
        {
            console_error!("Error disabling button: {}", error);
        }
    }
}

impl SimpleMessageResponse {
    #[allow(dead_code)]
    fn not_implemented() -> Self {
        Self {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: NoComponentMessage::not_implemented(),
        }
    }

    fn help() -> Self {
        Self {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: NoComponentMessage::help(),
        }
    }

    fn success() -> Self {
        Self {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: NoComponentMessage::success(),
        }
    }

    fn error() -> Self {
        Self {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: NoComponentMessage::error(),
        }
    }

    fn dms_closed() -> Self {
        Self {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: NoComponentMessage::dms_closed(),
        }
    }

    fn already_active() -> Self {
        Self {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: NoComponentMessage::already_active(),
        }
    }

    fn not_active() -> Self {
        Self {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: NoComponentMessage::not_active(),
        }
    }
}

impl SingleTextInputModalData {
    pub fn with_name(name: String) -> Self {
        Self {
            custom_id: ModalId::GratefulModal,
            title: format!("{}'s Gratitude Journal", name),
            components: [SingleTextInputActionRow::with_text_entry()],
        }
    }
}

impl TextInput {
    pub fn new() -> Self {
        TextInput {
            r#type: ModalComponentType::TextInput,
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

impl SingleButtonMessage {
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
        SingleButtonMessage {
            content,
            components: [SingleButtonActionRow::entry_button()],
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
        Self {
            content,
            components: [SingleButtonActionRow::entry_button()],
            ..Default::default()
        }
    }
}

impl NoComponentMessage {
    pub fn not_implemented() -> Self {
        Self {
            content: Some("This command is not yet implemented! Coming soon!".into()),
            flags: Some(1 << 6),
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
        Self {
            content,
            ..Default::default()
        }
    }

    pub fn help() -> Self {
        Self {
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

    pub fn success() -> Self {
        Self {
            content: Some(
                "**It looks like that worked! 🥳** If it didn't do what you expected, contact Fitti#6969"
                    .to_string(),
            ),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn error() -> Self {
        Self {
            content: Some(
                "Oh no! It looks like something went wrong!\nAsk Fitti#6969 for help!".into(),
            ),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn dms_closed() -> Self {
        Self {
            content: Some(format!(
                "It looks like the bot can't DM you! Check your privacy settings: {}",
                "https://support.discord.com/hc/en-us/articles/217916488-Blocking-Privacy-Settings",
            )),
            flags: Some(1 << 6),
            ..Default::default()
        }
    }

    pub fn already_active() -> Self {
        Self {
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
        Self {
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

impl SingleButtonActionRow {
    fn entry_button() -> Self {
        Self {
            r#type: ActionRowType::ActionRow,
            components: [Button::entry()],
        }
    }
}

impl SingleTextInputActionRow {
    fn with_text_entry() -> Self {
        Self {
            r#type: ActionRowType::ActionRow,
            components: [TextInput::new()],
        }
    }
}

impl Button {
    fn entry() -> Self {
        Button {
            r#type: InteractionComponentType::Button,
            style: 3,
            label: "What are you grateful for today?".into(),
            custom_id: CustomId::GratefulButton,
            disabled: Some(false),
        }
    }
}

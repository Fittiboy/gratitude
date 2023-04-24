use crate::discord;
use crate::interaction::{Message, NoComponentMessage, SimpleMessageResponse};
use crate::users::BotUser;
use std::fmt;
use worker::kv::{KvError, KvStore};
use worker::{console_error, console_log};

pub struct CommandHandler {
    //TODO: Add ApplicationCommandData for /entry
    pub user: BotUser,
    pub client: discord::Client,
    pub users_kv: KvStore,
    pub thankful_kv: KvStore,
    pub add_key: String,
    pub delete_key: String,
    pub users: Vec<BotUser>,
}

impl CommandHandler {
    pub async fn handle_start(&mut self) -> SimpleMessageResponse {
        console_log!("Handling start!");
        if self.delete_present().await {
            if let Err(err) = self.remove_delete().await {
                console_error!("Couldn't remove delete key from kv: {}", err);
                return SimpleMessageResponse::error();
            }
        } else if self.already_active().await {
            return SimpleMessageResponse::already_active();
        } else if let Err(err) = self.insert_add().await {
            console_error!("Couldn't add user to list: {}", err);
            return SimpleMessageResponse::error();
        }
        if let Err(error) = self.notify_start().await {
            console_error!("{}", error.to_string());
            return SimpleMessageResponse::dms_closed();
        }
        console_log!("New user: {:?}", self.user.uid);

        SimpleMessageResponse::success()
    }

    pub async fn handle_stop(&mut self) -> SimpleMessageResponse {
        console_log!("Handling stop!");
        if self.add_present().await {
            if let Err(err) = self.remove_add().await {
                console_error!("Couldn't remove delete key from kv: {}", err);
                return SimpleMessageResponse::error();
            }
        } else if self.not_active().await {
            return SimpleMessageResponse::not_active();
        } else if (self.drop_user().await).is_err() {
            return SimpleMessageResponse::error();
        } else if let Err(err) = self.insert_delete().await {
            console_error!("Couldn't remove user from list: {}", err);
            return SimpleMessageResponse::error();
        }
        if (self.notify_stop().await).is_err() {
            return SimpleMessageResponse::dms_closed();
        }
        console_log!("User removed: {:?}", self.user.uid);

        SimpleMessageResponse::success()
    }

    pub async fn handle_entry(&mut self, entry: &str) -> SimpleMessageResponse {
        if (self.notify_entry(entry).await).is_err() {
            return SimpleMessageResponse::dms_closed();
        }

        SimpleMessageResponse::success()
    }

    pub async fn delete_present(&self) -> bool {
        self.users_kv
            .get(&self.delete_key)
            .text()
            .await
            .unwrap()
            .is_some()
    }

    pub async fn add_present(&self) -> bool {
        self.users_kv
            .get(&self.add_key)
            .text()
            .await
            .unwrap()
            .is_some()
    }

    pub async fn remove_delete(&self) -> Result<(), KvError> {
        self.users_kv.delete(&self.delete_key).await
    }

    pub async fn remove_add(&self) -> Result<(), KvError> {
        self.users_kv.delete(&self.add_key).await
    }

    pub async fn insert_delete(&self) -> Result<(), KvError> {
        self.users_kv
            .put(&self.delete_key, "POOF")
            .unwrap()
            .execute()
            .await
    }

    pub async fn insert_add(&self) -> Result<(), KvError> {
        self.users_kv
            .put(&self.add_key, "FOOP")
            .unwrap()
            .execute()
            .await
    }

    pub async fn already_active(&self) -> bool {
        self.users.iter().any(|user| user.uid == self.user.uid)
            || self
                .users_kv
                .get(&self.add_key)
                .text()
                .await
                .unwrap()
                .is_some()
    }

    pub async fn not_active(&self) -> bool {
        !self.users.iter().any(|user| user.uid == self.user.uid)
            || self
                .users_kv
                .get(&self.delete_key)
                .text()
                .await
                .unwrap()
                .is_some()
    }

    pub async fn drop_user(&mut self) -> Result<(), CommandHandlerError> {
        let before = self.users.len();
        self.users.retain(|user| user.uid != self.user.uid);
        let after = self.users.len();
        if before - 1 != after {
            Err(CommandHandlerError::DropUser { before, after })
        } else {
            Ok(())
        }
    }

    pub async fn notify_start(&mut self) -> Result<(), CommandHandlerError<'_>> {
        let payload = Message::welcome();
        self.notify(payload).await
    }

    pub async fn notify_stop(&mut self) -> Result<(), CommandHandlerError<'_>> {
        let payload = Message::goodbye();
        self.notify(payload).await
    }

    pub async fn notify_entry(&mut self, entry: &str) -> Result<(), CommandHandlerError<'_>> {
        let payload = NoComponentMessage::from_entry(entry);
        self.notify(payload).await
    }

    pub async fn notify<T>(&mut self, payload: Message<T>) -> Result<(), CommandHandlerError<'_>>
    where
        T: serde::Serialize,
    {
        let client = self
            .client
            .post(&format!("channels/{}/messages", self.user.channel_id))
            .json(&payload);
        if let Err(error) = client.send().await.unwrap().error_for_status() {
            let error = CommandHandlerError::Notify {
                uid: &self.user.uid,
                error: error.to_string(),
            };
            console_error!("{}", error.to_string());
            Err(error)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum CommandHandlerError<'a> {
    DropUser { before: usize, after: usize },
    Notify { uid: &'a str, error: String },
}

impl fmt::Display for CommandHandlerError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DropUser { before, after } => {
                write!(
                    f,
                    "Length after removing not one less. Old: {}, New: {}",
                    before, after
                )
            }
            Self::Notify { uid, error } => {
                write!(f, "Error sending message to user {}: {}", uid, error)
            }
        }
    }
}

impl std::error::Error for CommandHandlerError<'_> {}

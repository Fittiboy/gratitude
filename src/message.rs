use crate::interaction::Message;
use crate::DiscordAPIClient;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use worker::kv::KvStore;
use worker::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub uid: String,
    pub channel_id: String,
}

impl User {
    pub async fn prompt(&self, kv: &KvStore, client: &mut DiscordAPIClient) {
        let entry = self.random_entry(kv).await;
        let payload = Message::from_entry(entry);

        let client = client
            .post(&format!("channels/{}/messages", self.channel_id))
            .json(&payload);
        console_log!("Prompting {}", self.uid);
        if let Err(error) = client.send().await.unwrap().error_for_status() {
            console_error!("Error sending message to user {}: {}", self.uid, error);
        }
    }

    async fn random_entry(&self, kv: &KvStore) -> Option<String> {
        let entries = kv.get(&self.uid).text().await.unwrap();
        console_log!("Entries: {:?}", entries);
        let entries: Vec<String> = match entries {
            Some(entries) => serde_json::from_str(&entries).unwrap(),
            None => Vec::new(),
        };
        let mut rng = rand::thread_rng();
        entries.choose(&mut rng).cloned()
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Journal {
    entries: Vec<String>,
}

pub async fn registered_users(users_kv: KvStore) -> Vec<User> {
    users_kv
        .get("users")
        .json::<Vec<User>>()
        .await
        .unwrap_or_else(|err| {
            console_error!("Couldn't parse string into vector of users: {}!", err);
            panic!()
        })
        .unwrap_or_else(|| {
            console_error!("No registered users!");
            panic!()
        })
        .into_iter()
        .collect()
}

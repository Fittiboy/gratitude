use crate::interaction::Message;
use crate::DiscordAPIBuilder;
use serde::{Deserialize, Serialize};
use worker::kv::KvStore;
use worker::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub uid: String,
    pub channel_id: String,
}

impl User {
    pub async fn prompt(&self, env: &Env) {
        let kv = env.kv("thankful").unwrap();
        console_log!("KV: {:?}", kv.list().execute().await.unwrap());
        let entries = kv.get(&self.uid).text().await.unwrap();
        console_log!("Entries: {:?}", entries);
        let entries: Vec<String> = match entries {
            Some(entries) => serde_json::from_str(&entries).unwrap(),
            None => Vec::new(),
        };
        //TODO actually grab random entry!
        let entry = entries.iter().next();
        let payload = Message::from_entry(entry.cloned());
        console_log!(
            "Payload: {}",
            serde_json::to_string_pretty(&payload).unwrap()
        );

        let client = DiscordAPIBuilder::new(&env)
            .post(&format!("channels/{}/messages", self.channel_id))
            .json(&payload);
        if let Err(error) = client.send().await.unwrap().error_for_status() {
            console_error!("Error posting message to me: {}", error);
        }
        console_log!("Prompting {}", self.uid);
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

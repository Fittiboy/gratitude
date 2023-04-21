use crate::discord;
use crate::interaction::Message;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use worker::kv::KvStore;
use worker::*;

pub async fn registered(kv: &KvStore) -> Vec<BotUser> {
    kv.get("users")
        .json::<Vec<BotUser>>()
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

pub async fn update(users: &mut Vec<BotUser>, kv: &kv::KvStore) {
    loop {
        let mut to_delete = Vec::new();
        let mut to_add = Vec::new();

        let todo = kv.list().execute().await.unwrap();

        let mut keys = todo.keys;
        keys.retain(|key| key.name != "users");
        for key in keys.as_slice() {
            match key.name {
                ref name if name.starts_with("DELETE") => {
                    let uid = name.as_str().split_once(' ').unwrap().1;
                    to_delete.push(uid.to_owned());
                }
                ref name if name.starts_with("ADD") => {
                    let user = name.as_str().split_once(' ').unwrap().1;
                    to_add.push(user.to_owned());
                }
                ref name => {
                    console_log!("Ignoring key: {}!", name);
                }
            }
        }
        for user in to_add.as_slice() {
            users.push(from_str::<BotUser>(user).unwrap());
        }
        users.retain(|user| !to_delete.contains(&user.uid));
        for key in keys {
            match kv.delete(&key.name).await {
                Ok(_) => console_log!("Removed key: {}", &key.name),
                Err(err) => console_error!("Couldn't remove user {}: {}", &key.name, err),
            };
        }
        match kv.put("users", &users) {
            Ok(task) => match task.execute().await {
                Ok(_) => console_log!("Updated users!"),
                Err(err) => console_error!("Couldn't update users {}", err),
            },
            Err(err) => console_error!("Couldn't update users: {}", err),
        }
        if todo.list_complete {
            break;
        }
    }
}

pub async fn prompt(users: &Vec<BotUser>, kv: &KvStore, client: &mut discord::Client) {
    let mut rng = thread_rng();
    let users = users.iter().filter(|_| rng.gen_range(1..=60) == 1);

    for user in users {
        user.prompt(&kv, client).await;
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BotUser {
    pub uid: String,
    pub channel_id: String,
}

impl BotUser {
    pub async fn prompt(&self, kv: &KvStore, client: &mut discord::Client) {
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
            Some(entries) => from_str(&entries).unwrap(),
            None => Vec::new(),
        };
        let mut rng = rand::thread_rng();
        entries.choose(&mut rng).cloned()
    }
}

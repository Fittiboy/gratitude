use serde::{Deserialize, Serialize};
use worker::kv::KvStore;
use worker::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    id: u64,
}

impl User {
    pub async fn prompt(&self) {
        console_log!("Prompting {:?}", self.id);
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Journal {
    entries: Vec<String>,
}

pub async fn registered_users(users_kv: KvStore) -> Vec<User> {
    users_kv
        .get("users")
        .json::<Vec<String>>()
        .await
        .unwrap_or_else(|err| {
            console_log!("Couldn't parse users into vector of strings: {}!", err);
            panic!()
        })
        .unwrap_or_else(|| {
            console_log!("Couldn't parse users into vector of strings!");
            panic!()
        })
        .into_iter()
        .map(|id| User {
            id: id.parse::<u64>().unwrap_or_else(|err| {
                console_log!("Couldn't convert uid string into u64: {}", err);
                panic!()
            }),
        })
        .collect()
}

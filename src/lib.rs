use rand::Rng;
use reqwest::{header, Client, RequestBuilder};
use worker::*;

mod bot;
mod commands;
mod embed;
mod error;
mod http;
mod interaction;
mod message;
mod utils;
mod verification;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    utils::set_panic_hook();
    utils::log_request(&req);

    let router = Router::new();
    router
        .post_async("/", |req, ctx| async move {
            let mut app = bot::App::new(req, ctx);

            match app.handle_request().await {
                Ok(result) => {
                    console_log!(
                        "Response : {}",
                        serde_json::to_string_pretty(&result).unwrap()
                    );
                    Response::from_json(&result)
                }
                Err(httperr) => {
                    console_error!("Error response : {}", httperr.to_string());
                    Response::error(httperr.to_string(), httperr.status as u16)
                }
            }
        })
        .run(req, env)
        .await
}

#[event(scheduled)]
pub async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let token = discord_token(&env).unwrap();
    let mut client = DiscordAPIClient::new(token);

    // use crate::commands::{ApplicationCommand, ApplicationCommandOption};
    // use crate::interaction::data_types::{CommandName, OptionType};
    // let application_id = discord_application_id(&env).unwrap();
    // ApplicationCommand {
    //     application_id: application_id.clone(),
    //     description: "Start receiving reminders from the bot!".into(),
    //     dm_permission: Some(true),
    //     ..Default::default()
    // }
    // .register(&mut client)
    // .await;
    // ApplicationCommand {
    //     name: CommandName::Stop,
    //     application_id: application_id.clone(),
    //     description: "Stop receiving reminders from the bot!".into(),
    //     dm_permission: Some(true),
    //     ..Default::default()
    // }
    // .register(&mut client)
    // .await;
    // ApplicationCommand {
    //     name: CommandName::Entry,
    //     description: "Add an entry to your gratitude journal!".into(),
    //     options: Some(vec![ApplicationCommandOption {
    //         r#type: OptionType::String,
    //         name: "entry".into(),
    //         description: "Something, anything, you are feeling grateful for!".into(),
    //         required: Some(true),
    //         min_length: Some(5),
    //         max_length: Some(1000),
    //     }]),
    //     application_id,
    //     dm_permission: Some(true),
    //     ..Default::default()
    // }
    // .register(&mut client)
    // .await;

    let users_kv = env
        .kv("grateful_users")
        .expect("Worker should have access to grateful_users binding");
    let entries_kv = env
        .kv("thankful")
        .expect("Worker should have access to thankful binding");

    let mut users = message::registered_users(&users_kv).await;
    let mut done = false;
    while !done {
        let mut to_delete = Vec::new();
        let mut to_add = Vec::new();

        let todo = users_kv.list().execute().await.unwrap();
        done = todo.list_complete;

        let mut keys = todo.keys;
        keys.retain(|key| key.name != "users");
        for key in keys.as_slice() {
            match key.name {
                ref name if name.starts_with("DELETE") => {
                    let uid = name.as_str().split_once(" ").unwrap().1;
                    to_delete.push(uid.to_owned());
                }
                ref name if name.starts_with("ADD") => {
                    let user = name.as_str().split_once(" ").unwrap().1;
                    to_add.push(user.to_owned());
                }
                ref name => {
                    console_error!("No other keys should be present, found {}!", name);
                    unreachable!();
                }
            }
        }
        for user in to_add.as_slice() {
            users.push(serde_json::from_str::<message::User>(user).unwrap());
        }
        users.retain(|user| !to_delete.contains(&user.uid));
        for key in keys {
            match users_kv.delete(&key.name).await {
                Ok(_) => console_log!("Removed key: {}", &key.name),
                Err(err) => console_error!("Couldn't remove user {}: {}", &key.name, err),
            };
        }
        match users_kv.put("users", &users) {
            Ok(task) => match task.execute().await {
                Ok(_) => console_log!("Updated users!"),
                Err(err) => console_error!("Couldn't update users {}", err),
            },
            Err(err) => console_error!("Couldn't update users: {}", err),
        }
    }

    let mut rng = rand::thread_rng();
    let users = message::registered_users(&users_kv).await;
    let users = users.iter().filter(|_| rng.gen_range(1..=24) == 1);

    for user in users {
        user.prompt(&entries_kv, &mut client).await;
    }
}

pub struct DiscordAPIClient {
    client: Client,
}

impl DiscordAPIClient {
    pub fn new(token: String) -> Self {
        let headers = Self::headers(token.clone());
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Self { client }
    }

    pub fn patch(&mut self, url: &str) -> RequestBuilder {
        let url = format!("https://discord.com/api/{}", url);
        self.client.patch(&url)
    }

    pub fn post(&mut self, url: &str) -> RequestBuilder {
        let url = format!("https://discord.com/api/{}", url);
        self.client.post(&url)
    }

    pub fn get(&mut self, url: &str) -> RequestBuilder {
        let url = format!("https://discord.com/api/{}", url);
        self.client.get(&url)
    }

    pub fn delete(&mut self, url: &str) -> RequestBuilder {
        let url = format!("https://discord.com/api/{}", url);
        self.client.delete(&url)
    }

    fn headers(token: String) -> header::HeaderMap {
        let mut headers = header::HeaderMap::new();
        let auth_value = header::HeaderValue::from_str(&token).unwrap();
        headers.insert(header::AUTHORIZATION, auth_value);
        headers
    }
}

pub fn discord_token(env: &Env) -> Result<String> {
    let discord_token = env.var("DISCORD_TOKEN")?.to_string();
    Ok("Bot ".to_string() + &discord_token)
}

pub fn discord_application_id(env: &Env) -> Result<String> {
    let application_id = env.var("DISCORD_APPLICATION_ID")?.to_string();
    Ok(application_id)
}

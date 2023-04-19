use rand::Rng;
use reqwest::{header, Client, RequestBuilder};
use worker::*;

mod bot;
mod commands;
mod durable;
mod embed;
mod error;
mod http;
mod interaction;
mod message;
mod utils;
mod verification;

pub use durable::Userlist;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();

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

    // use crate::commands::ApplicationCommand;
    // use crate::interaction::data_types::CommandName;
    // let application_id = discord_application_id(&env).unwrap();
    // ApplicationCommand {
    //     application_id: application_id.clone(),
    //     description: "Start receiving reminders from the bot!".into(),
    //     ..Default::default()
    // }
    // .register(&mut client)
    // .await;
    // ApplicationCommand {
    //     name: CommandName::Stop,
    //     application_id: application_id.clone(),
    //     description: "Stop receiving reminders from the bot!".into(),
    //     ..Default::default()
    // }
    // .register(&mut client)
    // .await;
    // ApplicationCommand {
    //     name: CommandName::Entry,
    //     description: "Add an entry to your gratitude journal!".into(),
    //     application_id,
    //     ..Default::default()
    // }
    // .register(&mut client)
    // .await;

    let entries_kv = env
        .kv("thankful")
        .expect("Worker should have access to thankful binding");

    let mut rng = rand::thread_rng();
    let userlist = match env.durable_object("USERS") {
        Ok(userlist) => userlist,
        Err(err) => {
            console_error!("Error: {:#?}", err);
            panic!();
        }
    };
    let userlist = match userlist.id_from_name("production") {
        Ok(userlist) => userlist,
        Err(err) => {
            console_error!("Error: {:#?}", err);
            panic!();
        }
    };
    let userlist = match userlist.get_stub() {
        Ok(userlist) => userlist,
        Err(err) => {
            console_error!("Error: {:#?}", err);
            panic!();
        }
    };
    //TODO: Migrations?
    let request_init = RequestInit::new();
    let request = match Request::new_with_init("/something", &request_init) {
        Ok(thing) => thing,
        Err(err) => {
            console_error!("Error: {:#?}", err);
            panic!();
        }
    };
    let mut response = match userlist.fetch_with_request(request).await {
        Ok(response) => response,
        Err(err) => {
            console_error!("Error: {:#?}", err);
            panic!();
        }
    };
    let users = match response.json::<Vec<message::User>>().await {
        Ok(users) => users,
        Err(err) => {
            console_error!("Error: {:#?}", err);
            panic!();
        }
    };
    let users = users.iter().filter(|_| rng.gen_range(1..=24) == 1);

    for user in users {
        console_log!("{:?}", user);
        // user.prompt(&entries_kv, &mut client).await;
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

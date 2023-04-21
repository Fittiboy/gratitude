use commands::update_commands;
use message::{prompt_users, registered_users, update_users};
use reqwest::{header, Client, RequestBuilder};
use worker::*;

mod bot;
mod commands;
mod embed;
mod error;
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
        .get_async("/", |_, _| async move {
            let url = reqwest::Url::parse(concat!(
                "https://discord.com/api/oauth2/authorize",
                "?client_id=1094831789442343002",
                "&permissions=1024",
                "&scope=applications.commands%20bot",
            ))?;
            Response::redirect_with_status(url, 308)
        })
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
    let users_kv = env
        .kv("grateful_users")
        .expect("Worker should have access to grateful_users binding");
    let mut users = registered_users(&users_kv).await;
    update_users(&mut users, &users_kv).await;

    let token = discord_token(&env).unwrap();
    let mut client = DiscordAPIClient::new(token);
    update_commands(&users_kv, &env, &mut client).await;

    let entries_kv = env
        .kv("thankful")
        .expect("Worker should have access to thankful binding");
    prompt_users(&users, &entries_kv, &mut client).await;
}

pub struct DiscordAPIClient {
    client: Client,
}

impl DiscordAPIClient {
    pub fn new(token: String) -> Self {
        let headers = Self::headers(token);
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Self { client }
    }

    pub fn patch(&mut self, url: &str) -> RequestBuilder {
        let url = format!("https://discord.com/api/{}", url);
        self.client.patch(url)
    }

    pub fn post(&mut self, url: &str) -> RequestBuilder {
        let url = format!("https://discord.com/api/{}", url);
        self.client.post(url)
    }

    pub fn get(&mut self, url: &str) -> RequestBuilder {
        let url = format!("https://discord.com/api/{}", url);
        self.client.get(url)
    }

    pub fn delete(&mut self, url: &str) -> RequestBuilder {
        let url = format!("https://discord.com/api/{}", url);
        self.client.delete(url)
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

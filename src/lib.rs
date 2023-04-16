use worker::*;

mod bot;
mod embed;
mod error;
mod http;
mod interaction;
mod message;
mod utils;
mod verification;

use crate::interaction::Message;

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
                    console_log!("Error response : {}", httperr.to_string());
                    Response::error(httperr.to_string(), httperr.status as u16)
                }
            }
        })
        .run(req, env)
        .await
}

#[event(scheduled)]
pub async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    // let discord_token = env.var("DISCORD_TOKEN").unwrap().to_string();
    // let discord_token = "Bot ".to_string() + &discord_token;
    // let chan_id = "1096015676134658089";
    // let payload = Message::new(None);
    // let client = reqwest::Client::new();
    // if let Err(error) = client
    //     .post(format!(
    //         "https://discord.com/api/channels/{}/messages",
    //         chan_id
    //     ))
    //     .header(reqwest::header::AUTHORIZATION, discord_token)
    //     .json(&payload)
    //     .send()
    //     .await
    //     .unwrap()
    //     .error_for_status()
    // {
    //     console_log!("Error posting message to me: {}", error);
    // }
    let users_kv = env
        .kv("grateful_users")
        .expect("Worker should have access to this binding");
    for user in message::registered_users(users_kv).await {
        user.prompt().await;
    }
}

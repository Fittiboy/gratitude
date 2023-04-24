#![allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]
use worker::*;

mod bot;
mod commands;
mod discord;
mod error;
mod interaction;
mod users;
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
                Ok(mut result) => {
                    let mut clone = result.cloned().unwrap();
                    console_log!("Response: {}", clone.text().await?);
                    Ok(result)
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
pub async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let users_kv = env
        .kv("grateful_users")
        .expect("Worker should have access to grateful_users binding");
    let mut users = users::registered(&users_kv).await;
    users::update(&mut users, &users_kv).await;

    let token = discord::token(&env).unwrap();
    let mut client = discord::Client::new(&token);
    commands::update(&env, &mut client).await;

    if event.cron() != *"TEST" {
        let entries_kv = env
            .kv("thankful")
            .expect("Worker should have access to thankful binding");
        users::prompt(&users, &entries_kv, &mut client).await;
    }
}

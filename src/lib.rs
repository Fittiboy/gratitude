use serde::{Deserialize, Serialize};
use worker::*;

mod bot;
mod embed;
mod error;
mod http;
mod interaction;
mod utils;
mod verification;

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

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .post_async("/", |req, ctx| async move {
            let mut app = bot::App::new(req, ctx);

            match app.handle_request().await {
                Ok(result) => {
                    worker::console_log!(
                        "Response : {}",
                        serde_json::to_string_pretty(&result).unwrap()
                    );
                    Response::from_json(&result)
                }
                Err(httperr) => {
                    worker::console_log!("Error response : {}", httperr.to_string());
                    Response::error(httperr.to_string(), httperr.status as u16)
                }
            }
        })
        .run(req, env)
        .await
}

#[derive(Debug, Deserialize, Serialize)]
struct User {
    active: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Journal {
    entries: Vec<String>,
}

#[event(scheduled)]
pub async fn scheduled(event: ScheduledEvent, _env: Env, _ctx: ScheduleContext) {
    console_log!("This is a scheduled event:\nEvent: {:#?}\n", event,);
    // let kv = env.kv("thankful").unwrap();

    // let users = match kv.get("names").json::<Vec<String>>().await {
    //     Ok(Some(users)) => {
    //         console_log!("{:?}", users);
    //         users
    //     }
    //     Ok(None) => {
    //         console_log!("No users!");
    //         Vec::new()
    //     }
    //     Err(error) => {
    //         console_log!("Couldn't read from KV store: {}", error);
    //         Vec::new()
    //     }
    // };

    // for user in users {
    //     let other = if user == "Fitti".to_string() {
    //         "John"
    //     } else {
    //         "Fitti"
    //     };
    //     if let Err(error) = kv.put(&user, vec![other, "Me"]).unwrap().execute().await {
    //         console_log!("Couldn't write to KV store: {}", error);
    //     }
    //     match kv.get(&user).json::<Vec<String>>().await {
    //         Ok(Some(words)) => console_log!("{}: {:?}", user, words),
    //         Ok(None) => console_log!("Found empty vector!"),
    //         Err(error) => console_log!("Couldn't read from KV store: {}", error),
    //     }
    // }
}

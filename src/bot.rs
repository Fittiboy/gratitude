use crate::discord;
use crate::error;
use crate::interaction::data_types::{InteractionVariants, PingInteraction};
use crate::verification::verify_signature;
use serde_json::from_str;
use worker::Response as Res;
use worker::{console_log, Request, RouteContext};

pub struct App {
    req: Request,
    ctx: RouteContext<()>,
}

impl App {
    pub fn new(req: Request, ctx: RouteContext<()>) -> App {
        App { req, ctx }
    }

    fn var(&self, key: &str) -> Result<String, error::General> {
        match self.ctx.var(key) {
            Ok(var) => Ok(var.to_string()),
            Err(_) => Err(error::General::EnvironmentVariableNotFound(key.to_string())),
        }
    }

    fn header(&self, key: &str) -> Result<String, error::General> {
        match self.req.headers().get(key) {
            Ok(val) => val.ok_or_else(|| error::General::HeaderNotFound(key.to_string())),
            Err(_) => Err(error::General::HeaderNotFound(key.to_string())),
        }
    }

    async fn validate_sig(&mut self) -> Result<String, error::General> {
        let pubkey = self.var("DISCORD_PUBLIC_KEY")?;
        let signature = self.header("x-signature-ed25519")?;
        let timestamp = self.header("x-signature-timestamp")?;

        let body = self
            .req
            .text()
            .await
            .map_err(|_| error::General::InvalidPayload(String::new()))?;
        verify_signature(&pubkey, &signature, &timestamp, &body)
            .map_err(error::General::VerificationFailed)?;
        Ok(body)
    }

    pub async fn handle_request(&mut self) -> Result<Res, error::Http> {
        let body = self.validate_sig().await?;
        let thankful_kv = self
            .ctx
            .env
            .kv("thankful")
            .expect("Worker should have access to thankful binding");
        let client = discord::Client::new(&discord::token(&self.ctx.env).unwrap());
        let users_kv = self
            .ctx
            .env
            .kv("grateful_users")
            .expect("Worker should have access to grateful_users binding");

        console_log!("Request body : {}", body);

        match from_str::<InteractionVariants>(&body).map_err(error::General::from)? {
            InteractionVariants::Ping(_) => Ok(Res::from_json(&PingInteraction::handle())?),
            InteractionVariants::Command(i) => Ok(Res::from_json(
                &i.handle(client, users_kv, thankful_kv).await,
            )?),
            InteractionVariants::Button(i) => Ok(Res::from_json(&i.handle_grateful())?),
            InteractionVariants::Modal(mut i) => {
                Ok(Res::from_json(&i.handle(thankful_kv, client).await)?)
            }
        }
    }
}

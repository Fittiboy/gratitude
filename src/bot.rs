use crate::discord;
use crate::error::Error;
use crate::interaction::data_types::*;
use crate::verification::verify_signature;
use http::HttpError;
use worker::Response as Res;
use worker::{console_log, Request, RouteContext};

mod http;

pub struct App {
    req: Request,
    ctx: RouteContext<()>,
}

impl App {
    pub fn new(req: Request, ctx: RouteContext<()>) -> App {
        App { req, ctx }
    }

    fn var(&self, key: &str) -> Result<String, Error> {
        match self.ctx.var(key) {
            Ok(var) => Ok(var.to_string()),
            Err(_) => Err(Error::EnvironmentVariableNotFound(key.to_string())),
        }
    }
    fn header(&self, key: &str) -> Result<String, Error> {
        match self.req.headers().get(key) {
            Ok(val) => val.ok_or_else(|| Error::HeaderNotFound(key.to_string())),
            Err(_) => Err(Error::HeaderNotFound(key.to_string())),
        }
    }

    async fn validate_sig(&mut self) -> Result<String, Error> {
        let pubkey = self.var("DISCORD_PUBLIC_KEY")?;
        let signature = self.header("x-signature-ed25519")?;
        let timestamp = self.header("x-signature-timestamp")?;

        let body = self
            .req
            .text()
            .await
            .map_err(|_| Error::InvalidPayload(String::new()))?;
        verify_signature(&pubkey, &signature, &timestamp, &body)
            .map_err(Error::VerificationFailed)?;
        Ok(body)
    }

    pub async fn handle_request(&mut self) -> Result<Res, HttpError> {
        let body = self.validate_sig().await?;
        let thankful_kv = self
            .ctx
            .env
            .kv("thankful")
            .expect("Worker should have access to thankful binding");
        let mut client = discord::Client::new(&discord::token(&self.ctx.env).unwrap());
        let users_kv = self
            .ctx
            .env
            .kv("grateful_users")
            .expect("Worker should have access to grateful_users binding");

        console_log!("Request body : {}", body);

        match InteractionIdentifier::from_str(&body)?.r#type {
            InteractionType::Ping => Ok(Res::from_json(&PingInteraction::handle())?),
            InteractionType::ApplicationCommand => Ok(Res::from_json(
                &CommandInteraction::from_str(&body)?
                    .handle(client, users_kv, thankful_kv)
                    .await,
            )?),
            InteractionType::MessageComponent => {
                match ComponentIdentifier::from_str(&body)?.data.custom_id {
                    CustomId::GratefulButton => Ok(Res::from_json(
                        &ButtonInteraction::from_str(&body)?.handle_grateful(),
                    )?),
                }
            }
            InteractionType::ModalSubmit => Ok(Res::from_json(
                &SingleTextModalButtonInteraction::from_str(&body)?
                    .handle(thankful_kv, &mut client)
                    .await,
            )?),
        }
    }
}

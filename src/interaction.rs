use worker::{console_error, console_log, kv::KvStore, Env};

use crate::error::Error;
use crate::DiscordAPIBuilder;

mod data_types;
pub use data_types::*;

impl Interaction {
    pub async fn perform(
        &self,
        ctx: &mut worker::RouteContext<()>,
    ) -> Result<InteractionResponse, Error> {
        match self.r#type {
            InteractionType::Ping => Ok(self.handle_ping()),
            InteractionType::MessageComponent => Ok(self.handle_component()),
            InteractionType::ModalSubmit => Ok(self.handle_modal(&ctx.env).await),
        }
    }

    fn handle_ping(&self) -> InteractionResponse {
        InteractionResponse {
            r#type: InteractionResponseType::Pong,
            data: None,
        }
    }

    fn handle_component(&self) -> InteractionResponse {
        let InteractionData::ComponentInteractionData(component) = &self
            .data
            .as_ref()
            .expect("Component data should always be part of the interaction") else {
                console_error!("handle_component should only ever receive component data");
                unreachable!();
            };
        match component.component_type {
            ComponentType::Button => self.handle_button(&component.custom_id),
            _ => unimplemented!(
                "There are currently not other component types in use in this context"
            ),
        }
    }

    fn handle_button(&self, custom_id: &CustomId) -> InteractionResponse {
        match custom_id {
            CustomId::GratefulButton => self.handle_grateful_button(),
            _ => unreachable!("All button IDs are covered!"),
        }
    }

    fn handle_grateful_button(&self) -> InteractionResponse {
        let name = self
            .user
            .clone()
            .expect("Only users can click buttons")
            .username;
        console_log!("Handling button!");
        InteractionResponse {
            r#type: InteractionResponseType::Modal,
            data: Some(InteractionResponseData::Modal(Modal::with_name(name))),
        }
    }

    async fn handle_modal(&self, env: &Env) -> InteractionResponse {
        let entry = self.entry();
        self.add_entry(env, &entry).await;
        self.disable_button(env).await;

        InteractionResponse {
            r#type: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::Message(Message {
                id: None,
                channel_id: None,
                content: Some(format!("You said: {}", entry)),
                components: Some(vec![]),
            })),
        }
    }

    fn entry(&self) -> String {
        let action_row = self.modal_action_row();
        let action_row = action_row.components.iter().next().unwrap();
        let Component::TextInputSubmit(TextInputSubmit { value, .. }) =
            action_row.components.iter().next().unwrap() else {
                unreachable!("Modals support only text inputs");
            };
        value.to_owned()
    }

    async fn add_entry(&self, env: &Env, entry: &str) {
        let id = &self
            .user
            .clone()
            .expect("only users can interact with modals")
            .id;
        let kv = env
            .kv("thankful")
            .expect("worker should have binding to thankful namespace");
        let mut entries = self.get_entries(&kv, &id).await;
        entries.push(entry.to_string());
        kv.put(id, entries)
            .unwrap()
            .execute()
            .await
            .expect("should be able to serialize entries");
    }

    async fn get_entries(&self, kv: &KvStore, id: &String) -> Vec<String> {
        match kv.get(id).text().await {
            Ok(Some(text)) => serde_json::from_str(&text).unwrap(),
            Ok(None) => Vec::new(),
            Err(err) => {
                console_error!("Couldn't get entries: {}", err);
                panic!();
            }
        }
    }

    fn modal_action_row(&self) -> ModalSubmitData {
        match self
            .data
            .as_ref()
            .expect("Modal interactions always have data")
        {
            InteractionData::ModalInteractionData(ref data) => data.clone(),
            _ => unreachable!("Modal type is guaranteed at this point"),
        }
    }

    fn id_and_payload(&self) -> (String, MessageEdit) {
        let message = self.message.as_ref().unwrap();
        let message_id = message.id.clone().unwrap();
        let payload = message
            .components
            .clone()
            .expect("Messages with a modal always have at least one component");
        (
            message_id,
            MessageEdit {
                components: payload,
            },
        )
    }

    async fn disable_button(&self, env: &Env) {
        let (message_id, mut payload) = self.id_and_payload();
        Self::prepare_button_disable_payload(&mut payload);
        console_log!("Payload to disable button: {:#?}", payload);

        self.submit_disable_button_request(message_id, env, payload)
            .await;
    }

    fn prepare_button_disable_payload(payload: &mut MessageEdit) {
        let components = &mut payload.components;
        match components
            .first_mut()
            .unwrap()
            .components
            .first_mut()
            .unwrap()
        {
            Component::Button(Button { disabled, .. }) => *disabled = Some(true),
            _ => {}
        }
    }

    async fn submit_disable_button_request(
        &self,
        message_id: String,
        env: &Env,
        payload: MessageEdit,
    ) {
        let channel_id = self.channel_id.clone().unwrap();
        let client = DiscordAPIBuilder::new(env)
            .patch(&format!("channels/{}/messages/{}", channel_id, message_id,));
        if let Err(error) = client
            .json(&payload)
            .send()
            .await
            .unwrap()
            .error_for_status()
        {
            console_error!("Error disabling button: {}", error);
        }
    }
}

impl Modal {
    pub fn with_name(name: String) -> Self {
        Modal {
            custom_id: CustomId::GratefulModal,
            title: format!("{}'s Gratitude Journal", name),
            components: vec![ActionRow::with_text_entry()],
        }
    }
}

impl TextInput {
    pub fn new() -> Self {
        TextInput {
            r#type: ComponentType::TextInput,
            custom_id: CustomId::GratefulInput,
            style: 2,
            label: "Express your gratitude for something!".into(),
            min_length: 5,
            max_length: 1000,
            placeholder:
                "Today, I am grateful for… (a nice meal, someone smiling at me, how I perfectly parked my car)"
                    .to_string(),
        }
    }
}

impl Message {
    pub fn from_entry(journal_entry: Option<String>) -> Self {
        let content = match journal_entry {
            Some(text) => Some(format!(
                "*Here's something you were grateful for in the past:*\n{}",
                text
            )),
            None => Some("Hi there, welcome to gratitude bot!".into()),
        };
        Message {
            id: None,
            channel_id: None,
            content,
            components: Some(vec![ActionRow::with_entry_button()]),
        }
    }
}

impl ActionRow {
    fn with_entry_button() -> Self {
        ActionRow {
            r#type: ComponentType::ActionRow,
            components: vec![Component::Button(Button::entry())],
        }
    }

    fn with_text_entry() -> Self {
        ActionRow {
            r#type: ComponentType::ActionRow,
            components: vec![Component::TextInput(TextInput::new())],
        }
    }
}

impl Button {
    fn entry() -> Self {
        Button {
            r#type: ComponentType::Button,
            style: 3,
            label: "What are you grateful for today?".into(),
            custom_id: CustomId::GratefulButton,
            disabled: Some(false),
        }
    }
}

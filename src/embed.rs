use serde::Serialize;

#[derive(Serialize)]
pub struct Thumbnail {
    pub url: String,
}

#[derive(Serialize)]
pub struct EmbedFooter {
    pub text: String,
}

#[derive(Serialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: Option<bool>,
}

#[derive(Serialize)]
pub struct Embed {
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub thumbnail: Thumbnail,
    pub footer: Option<EmbedFooter>,
    pub fields: Vec<EmbedField>,
}

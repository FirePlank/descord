use json::object;
use nanoserde::{DeJson, SerJson};

use crate::utils;
use crate::{consts, Client};

use super::channel::Channel;
use super::message_edit::MessageEditData;
use super::{author::Author, embed::Embed, message_reference::MessageReference};

#[derive(DeJson, SerJson, Clone)]
pub struct MessageResponse {
    #[nserde(rename = "d")]
    pub data: MessageData,
}

#[derive(DeJson, SerJson, Clone)]
pub struct MessageData {
    #[nserde(default)]
    pub tts: bool,

    #[nserde(default)]
    pub timestamp: Option<String>,

    #[nserde(default)]
    pub pinned: bool,
    #[nserde(default)]
    pub mention_everyone: bool,

    #[nserde(default)]
    pub flags: usize,
    pub edited_timestamp: Option<String>,
    pub content: String,
    pub channel_id: String,
    pub embeds: Vec<Embed>,
    pub author: Author,

    #[nserde(default)]
    pub referenced_message: Option<MessageReference>,

    pub guild_id: Option<String>,

    #[nserde(rename = "id")]
    pub message_id: String,
    // TODO
    // mentions, mention_roles, member, etc.
}

impl MessageData {
    pub async fn reply(&self, data: impl Into<CreateMessageData>) -> MessageData {
        utils::reply(&self.message_id, &self.channel_id, data).await
    }

    pub async fn send_in_channel(&self, data: impl Into<CreateMessageData>) {
        utils::send(&self.channel_id, data).await;
    }

    pub async fn get_channel(&self) -> Result<Channel, Box<dyn std::error::Error>> {
        utils::get_channel(&self.channel_id).await
    }

    pub async fn delete(&self) -> bool {
        utils::delete_message(&self.channel_id, &self.message_id).await
    }

    pub async fn delete_after(&self, time: u64) {
        tokio::time::sleep(tokio::time::Duration::from_millis(time)).await;
        self.delete().await;
    }

    pub async fn edit(&self, data: impl Into<MessageEditData>) {
        utils::edit_message(&self.channel_id, &self.message_id, data).await;
    }
}

#[derive(Default, Debug, SerJson)]
pub struct CreateMessageData {
    pub content: String,
    pub tts: bool,

    // TODO: add max check
    pub embeds: Vec<Embed>,
}

impl From<String> for CreateMessageData {
    fn from(value: String) -> Self {
        Self {
            content: value,
            ..Default::default()
        }
    }
}

impl From<&str> for CreateMessageData {
    fn from(value: &str) -> Self {
        Self {
            content: value.to_owned(),
            ..Default::default()
        }
    }
}

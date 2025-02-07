use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use json::object;
use nanoserde::SerJson;

use crate::consts;
use crate::consts::intents::GatewayIntent;
use crate::handlers::EventHandler;
use crate::internals::*;
use crate::prelude::{CreateMessageData, MessageData};
use crate::ws::WsManager;

lazy_static::lazy_static! {
    pub(crate) static ref TOKEN: Mutex<Option<String>> = Mutex::new(None);
}

pub struct Client {
    intents: u32,
    ws: WsManager,
    token: String,
    commands: HashMap<String, Command>,
    prefix: String,
}

impl Client {
    pub async fn new(token: &str, intents: impl Into<u32>, prefix: &str) -> Self {
        *TOKEN.lock().unwrap() = Some(token.to_owned());

        Self {
            intents: intents.into(),
            token: token.to_owned(),
            ws: WsManager::new(token)
                .await
                .expect("Failed to initialize websockets"),
            commands: HashMap::new(),
            prefix: prefix.to_owned(),
        }
    }

    pub async fn login(self, event_handler: impl EventHandler + std::marker::Sync + 'static) {
        self.ws
            .connect(self.intents, event_handler.into(), self.commands.into())
            .await;
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn register_commands<const N: usize>(&mut self, commands: [Command; N]) {
        commands.into_iter().for_each(|mut command| {
            // if a custom prefix is not applied, add the default prefix
            if !command.custom_prefix {
                command.name = format!(
                    "{default_prefix}{name}",
                    default_prefix = self.prefix,
                    name = command.name
                );
            }

            self.commands.insert(command.name.clone(), command);
        });
    }
}

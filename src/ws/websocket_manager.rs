use log::*;
use nanoserde::DeJson;

use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{clone, thread};

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{future, pin_mut, SinkExt, StreamExt};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use tokio_tungstenite::tungstenite::{Message, Result};
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::{connect_async, WebSocketStream};
use url::Url;

use crate::client::Context;
use crate::consts::opcode::OpCode;
use crate::consts::{self, payloads};
use crate::handlers::events::Event;
use crate::handlers::EventHandler;
use crate::ws::payload::Payload;
use crate::{models::*, Client};

pub struct WsManager {
    ctx: Arc<Context>,
    token: String,
    socket: (
        Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
        Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    ),
}

impl WsManager {
    pub async fn new(token: &str) -> Result<Self> {
        let (socket, _response) = connect_async(Url::parse(consts::GATEWAY_URL).unwrap()).await?;

        let (write, read) = socket.split();
        let (write, read) = (Arc::new(Mutex::new(write)), Arc::new(Mutex::new(read)));

        Ok(Self {
            ctx: Arc::new(Context::new(token.to_string())),
            token: token.to_owned(),
            socket: (write, read),
        })
    }

    pub async fn connect<'a>(
        &'a self,
        intents: u32,
        event_handler: Arc<impl EventHandler + std::marker::Sync + 'static>,
    ) -> Result<()> {
        if let Some(Ok(Message::Text(body))) = self.socket.1.lock().unwrap().next().await {
            let Some(payload) = Payload::parse(&body) else {
                panic!("Failed to parse json, body: {body}");
            };

            match payload.operation_code {
                OpCode::Hello => {
                    info!("starting heartheat");
                    let time_ms = payload.data["heartbeat_interval"].as_u64().unwrap();
                    let writer = Arc::clone(&self.socket.0);

                    tokio::spawn(async move {
                        Self::heartbeat_start(Duration::from_millis(time_ms), writer);
                    });

                    info!("performing handshake");
                    self.identify(intents).await?;
                }

                _ => panic!("Unknown event received when attempting to handshake"),
            }
        }

        while let Some(Ok(Message::Text(body))) = self.socket.1.lock().unwrap().next().await {
            let Some(payload) = Payload::parse(&body) else {
                error!("Failed to parse json");
                continue;
            };

            match payload.operation_code {
                OpCode::Dispatch => {
                    let event_handler = Arc::clone(&event_handler);
                    let ctx = Arc::clone(&self.ctx);

                    info!(
                        "received {} event",
                        payload
                            .type_name
                            .as_ref()
                            .map(|i| i.as_str())
                            .unwrap_or("Unknown")
                    );

                    tokio::spawn(async move {
                        Self::dispatch_event(payload, event_handler, ctx).await;
                    });
                }

                _ => {}
            }
        }

        Ok(())
    }

    async fn dispatch_event(
        payload: Payload,
        event_handler: Arc<impl EventHandler>,
        ctx: Arc<Context>,
    ) {
        let event = Event::from_str(payload.type_name.as_ref().unwrap().as_str()).unwrap();

        match event {
            Event::Ready => {
                let ready_data = ready_response::ReadyResponse::deserialize_json(&payload.raw_json)
                    .expect("Failed to parse json");

                event_handler.ready(&ctx, ready_data.data).await;

                // const READY_SEQ: usize = 1;
                // if payload.sequence == Some(READY_SEQ) {
                // *self.session_id.lock().unwrap() = Some(ready_data.data.session_id.clone());
                // *self.resume_gateway_url.lock().unwrap() = Some(format!(
                //     "{}/?v=10&encoding=json",
                //     ready_data.data.resume_gateway_url
                // ));
                // }
            }

            Event::MessageCreate => {
                let ready_data =
                    message_response::MessageResponse::deserialize_json(&payload.raw_json)
                        .expect("Failed to parse json");

                event_handler.message_create(&ctx, ready_data.data).await;
            }

            _ => error!("{event:?} event is not implemented"),
        }
    }

    async fn heartbeat_start(
        heartbeat_interval: Duration,
        writer: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    ) {
        let mut last_sequence: usize = 0;
        loop {
            let message = Message::Text(json::stringify(payloads::heartbeat(last_sequence)));
            writer
                .lock()
                .unwrap()
                .send(message)
                .await
                .expect("Failed to send heartbeat");

            tokio::time::sleep(heartbeat_interval).await;
            last_sequence += 1;
        }

        // let socket = Arc::clone(&self.socket);
        // let resume_gateway_url = Arc::clone(&self.resume_gateway_url);
        // let session_id = Arc::clone(&self.session_id);
        // let last_sequence = Arc::clone(&self.last_sequence);
        // let token = self.token.clone();

        // loop {
        // info!("sending heartbeat");
        // if let Err(tungstenite::Error::AlreadyClosed) =
        //     socket
        //         .lock()
        //         .unwrap()
        //         .send(Message::Text(json::stringify(payloads::heartbeat(
        //             last_sequence.lock().unwrap().unwrap_or(0),
        //         ))))
        // {
        //     warn!("connection closed");
        //     info!("Reopening the connection...");
        //     let (mut socket, _response) = connect(
        //         Url::parse(
        //             resume_gateway_url
        //                 .lock()
        //                 .unwrap()
        //                 .as_ref()
        //                 .unwrap()
        //                 .as_str(),
        //         )
        //         .unwrap(),
        //     )
        //     .unwrap();

        //     socket
        //         .send(Message::Text(json::stringify(payloads::resume(
        //             &token,
        //             session_id.lock().unwrap().as_ref().unwrap().as_str(),
        //             last_sequence.lock().unwrap().unwrap(),
        //         ))))
        //         .unwrap();
        // }

        // thread::sleep(heartbeat_interval);
        // }
    }

    async fn identify(&self, intents: u32) -> Result<()> {
        self.send_text(json::stringify(payloads::identify(&self.token, intents)))
            .await
    }

    async fn send_text(&self, msg: String) -> Result<()> {
        self.socket.0.lock().unwrap().send(Message::Text(msg)).await
    }
}

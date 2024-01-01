use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use crate::{
    events::{EventQueue, EventSender},
    game::Player,
    message::{GameState, ToClient, ToServer},
    server,
    utils::{MessageReader, MessageWriter},
};
use futures_util::{SinkExt, StreamExt};

type ClientMessageReader = MessageReader<ToServer>;
type ClientMessageWriter = MessageWriter<ToClient>;

pub struct ClientSession {
    player: Player,
    peer_addr: SocketAddr,
    server: EventSender<server::Event>,
    client_msg_stream: (ClientMessageReader, ClientMessageWriter),
    event_queue: EventQueue<ToClient>,
    stop: bool,
}

impl ClientSession {
    pub fn new(
        id: Player,
        peer_addr: SocketAddr,
        server: EventSender<server::Event>,
        client_msg_stream: (ClientMessageReader, ClientMessageWriter),
    ) -> Self {
        Self {
            player: id,
            peer_addr,
            server,
            client_msg_stream,
            event_queue: EventQueue::default(),
            stop: false,
        }
    }

    pub fn sender(&self) -> &EventSender<ToClient> { self.event_queue.sender() }

    async fn send(&mut self, msg: ToClient) {
        if let Err(err) = self.client_msg_stream.1.send(msg).await {
            println!("{}", err);
            self.stop = true;
        }
    }

    async fn kick(&mut self, reason: String) {
        if !self.stop {
            self.stop = true;
            self.send(ToClient::Disconnect(reason)).await;
        }
    }

    pub async fn start(&mut self, initial_state: GameState) {
        struct CheckHeartBeat;
        let mut hb_check: EventQueue<CheckHeartBeat> = EventQueue::default();
        let mut last_hb = Instant::now();

        // send player initial game state
        self.send(ToClient::InitialState(self.player, initial_state))
            .await;

        let timeout_duration = Duration::from_secs(5);
        hb_check
            .sender()
            .send_with_delay(CheckHeartBeat, timeout_duration);

        while !self.stop {
            let client_msg = self.client_msg_stream.0.next();
            let server_msg = self.event_queue.recv_async();

            tokio::select! {
                _  = hb_check.recv_async() => {
                    if Instant::now().duration_since(last_hb)
                        > timeout_duration
                        {
                            // heartbeat timed out
                            log::warn!(
                                "({}): Client heartbeat failed, disconnecting!",
                                self.peer_addr
                                );

                            let _ = self.send(ToClient::Disconnect("Heartbeat failed".to_owned())).await;
                            break;
                        }
                },

                Some(msg) = server_msg => {
                    match msg {
                        ToClient::Disconnect(reason) => self.kick(reason).await,
                        _ => self.send(msg).await
                    }
                },

                Some(msg) = client_msg => {
                    match msg {
                        Ok(msg) =>  {
                            match msg {
                                ToServer::Heartbeat => {
                                    last_hb = Instant::now();
                                    hb_check.sender().send_with_delay(CheckHeartBeat, timeout_duration);
                                },
                                ToServer::Leave => break,
                                ToServer::PassBall(receiver) => self.server.send(server::Event::PassBall(self.player, receiver)),
                            };
                        }
                        Err(err) => {
                            log::error!("decode err {:?}", err);
                            break;
                        }
                    }
                }
                else => break,
            }
        }
        self.stop = true;

        // notify server
        self.server.send(server::Event::PlayerLeft(self.player));
    }
}

use argh::FromArgs;
use futures_util::StreamExt;
use std::{collections::HashMap, net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream};

use crate::{
    events::{EventQueue, EventSender},
    game::{GameInfo, Player},
    message::{GameState, ToClient, WhoPassed},
    session::ClientSession,
    utils,
};

/// run server
#[derive(FromArgs)]
#[argh(subcommand, name = "server")]
pub struct CliOpts {
    /// port for server to run on
    #[argh(option, short = 'p', default = "4565")]
    pub port: u16,
}

pub enum Event {
    CtrlC,
    PlayerLeft(Player),
    PassBall(Player, Player),
}

#[derive(Default)]
pub struct GameServer {
    connected_players: HashMap<Player, EventSender<ToClient>>,
    event_queue: EventQueue<Event>,
    id_counter: usize,
    game: GameInfo,
}

impl GameServer {
    pub fn sender(&self) -> &EventSender<Event> { self.event_queue.sender() }

    fn players(&self) -> Vec<Player> { self.connected_players.keys().cloned().collect() }

    fn gen_unique_id(&mut self) -> Player {
        self.id_counter += 1;
        Player(self.id_counter)
    }

    fn on_client_disconnect(&mut self, player_leaving: Player) {
        if let Some(_player_session) = self.connected_players.remove(&player_leaving) {
            println!("{} left the game!", player_leaving);

            let player_with_ball = self.game.player_with_ball.unwrap();
            if player_leaving == player_with_ball {
                if let Some(next_player) = self.connected_players.keys().next().cloned() {
                    print!(".. passing ball to another player ...");
                    self.pass_ball(next_player);
                } else {
                    self.game.player_with_ball = None;
                }
            }

            println!("players left in game: {:?}", self.players());
        }
    }

    /// handle stream of TcpStream
    fn on_client_connect(&mut self, peer_addr: SocketAddr, st: TcpStream) {
        let player = self.gen_unique_id();
        let sender = self.sender().clone();
        let socket = utils::frame_socket(st);

        let players = self.players();
        let game = self.game.clone();
        let initial_state = GameState {
            players,
            info: game,
        };

        let mut session = ClientSession::new(player, peer_addr, sender, socket);
        let session_sender = session.sender().clone();
        tokio::spawn(async move { session.start(initial_state).await });

        self.connected_players.insert(player, session_sender);

        // update all users on this user
        println!("{} joined the game!", player);
        log::info!("\t players in game: {:?}", self.players());
        self.broadcast(ToClient::PlayerJoin(player));

        if self.game.player_with_ball.is_none() {
            // ball will be given to first joining player
            self.pass_ball(player)
        }
    }

    fn pass_ball(&mut self, receiving: Player) {
        let who = if let Some(who_passed) = &self.game.player_with_ball {
            if self.connected_players.contains_key(who_passed) {
                println!(
                    "{} passed the bass -> {}",
                    who_passed,
                    who_passed.as_ref_str(&receiving)
                );
                WhoPassed::Player
            } else {
                println!("(system) ball passed -> {}", receiving);
                WhoPassed::PlayerWithBallLeft
            }
        } else {
            println!("(system) first player: ball passed -> {}", receiving);
            WhoPassed::PlayerStumbledUponBall
        };

        if self.connected_players.contains_key(&receiving) {
            self.game.player_with_ball = Some(receiving);
            self.broadcast(ToClient::PassBall(receiving, who));
        } else {
            println!("failed to pass the ball to {}", receiving);
        }
    }

    fn on_ball_pass(&mut self, sender: Player, receiving: Player) {
        if let Some(player_with_ball) = &self.game.player_with_ball {
            if player_with_ball != &sender {
                println!(
                    "{} tried to pass the ball to {} but doesn't have the ball",
                    sender,
                    sender.as_ref_str(&receiving)
                );
                return;
            }
        }

        self.pass_ball(receiving);
    }

    fn broadcast(&self, msg: ToClient) {
        self.connected_players.iter().for_each(|(_, player)| {
            player.send(msg.clone());
        });
    }

    /// start server listener on given address
    pub async fn run(mut self, addr: &str) -> Result<(), std::io::Error> {
        let mut tcp_listener = TcpListener::bind(addr).await?.map(|stream| {
            let st = stream.unwrap();
            let addr = st.peer_addr().unwrap();

            st.set_nodelay(true)
                .expect("Failed to set stream as nonblocking");

            st.set_keepalive(Some(Duration::from_secs(1)))
                .expect("Failed to set keepalive");

            (st, addr)
        });

        println!("🚀 Running game server on {}...", addr);

        loop {
            tokio::select! {
                Some(event) = self.event_queue.recv_async() => {
                    match event {
                        Event::CtrlC => break,
                        Event::PlayerLeft(player_id) => self.on_client_disconnect(player_id),
                        Event::PassBall(sender, receiver) => self.on_ball_pass(sender, receiver),
                    }
                }

                // listen and accept incoming connections in async thread.
                Some((socket, addr)) = tcp_listener.next() => self.on_client_connect(addr, socket),

                // tcp pipe probably closed, stop server
                else => break,
            };
        }

        // disconnect players
        for (_, player) in self.connected_players.drain() {
            player.send_with_urgency(ToClient::Disconnect("Server Shutdown".into()));
        }

        Ok(())
    }
}

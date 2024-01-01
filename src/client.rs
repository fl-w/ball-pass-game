use std::{io::BufRead, time::Duration};

use argh::FromArgs;
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use tokio::net::TcpStream;

use crate::{
    events::{EventQueue, EventSender},
    game::Player,
    message::{GameState, ToClient, ToServer, WhoPassed},
    utils,
};

/// run client
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "client")]
pub struct CliOpts {
    /// address of server to connect to.
    #[argh(option, short = 'h')]
    pub server_addr: String,
}

pub enum Event {
    Input(String),
    ServerMessage(ToClient),
    ConnectionDropped,
    CtrlC,
}

struct Game {
    myself: Player,
    state: GameState,
}

pub struct ClientApp {
    event_queue: EventQueue<Event>,
    server_tx: EventSender<ToServer>,
    game: Option<Game>,
    // should_exit: bool,
}

pub async fn start(addr: String) -> ClientApp {
    let event_queue: EventQueue<Event> = EventQueue::default();
    let app_tx = event_queue.sender().clone();

    ClientApp {
        event_queue,
        game: None,
        // should_exit: false,
        server_tx: connect_to_server(addr, app_tx).await,
    }
}

impl ClientApp {
    pub fn sender(&self) -> EventSender<Event> { self.event_queue.sender().clone() }

    pub async fn run_loop(&mut self) {
        let sender = self.event_queue.sender().clone();

        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            let mut iter = stdin.lock().lines();

            while let Some(Ok(line)) = iter.next() {
                sender.send(Event::Input(line));
            }

            sender.send_with_urgency(Event::CtrlC);
        });

        loop {
            match self.event_queue.recv_async().await.unwrap() {
                // handle server message
                Event::ServerMessage(server_msg) => match server_msg {
                    ToClient::InitialState(player, state) => {
                        self.game = Some(Game {
                            myself: player,
                            state,
                        })
                    }
                    ToClient::PlayerJoin(pl) => {
                        if let Some(ref mut game) = self.game {
                            println!("{} joined the game.", pl);
                            game.state.players.push(pl)
                        }
                    }
                    ToClient::PlayerLeave(pl) => {
                        if let Some(ref mut game) = self.game {
                            println!("(pass) {} -> SERVER", pl);
                            println!("{} left the game.", pl);
                            game.state.players.retain(|opl| opl == &pl)
                        }
                    }
                    ToClient::PassBall(to, who) => {
                        if let Some(game) = &mut self.game {
                            let from = game.state.info.player_with_ball;
                            game.state.info.player_with_ball.replace(to);
                            match who {
                                WhoPassed::Player => {
                                    println!("[PASS] {} -> {}", from.unwrap(), to)
                                }
                                WhoPassed::PlayerWithBallLeft => {
                                    println!("[PASS] SERVER -> {} (auto pass)", to,)
                                }
                                WhoPassed::PlayerStumbledUponBall => {
                                    println!("[PASS] SERVER -> you");
                                    println!("✨ You have the ball!");
                                }
                            };
                        }
                    }
                    ToClient::Disconnect(reason) => {
                        println!("You were disconnected from server: {}", reason)
                    }
                },

                // handle input events
                Event::Input(input) => {
                    let args: Vec<&str> = input.split(" ").collect();
                    if !args.is_empty() && args[0] == "pass" {
                        if let Ok(id) = args[1].parse() {
                            self.server_tx.send(ToServer::PassBall(Player(id)));
                            continue;
                        }
                    }
                    println!("invalid input: enter pass <id> or ctrlc to stop")
                }

                // handle server connection drop
                Event::ConnectionDropped => {
                    println!("✨ Server connection dropped. Stopping..");
                    break;
                }

                // close on ctrl-c
                Event::CtrlC => {
                    self.server_tx.send_with_urgency(ToServer::Leave);
                    break;
                    // stop
                }
            }
        }
    }
}

async fn connect_to_server(
    server_addr: String,
    app_tx: EventSender<Event>,
) -> EventSender<ToServer> {
    let mut server_msg_queue: EventQueue<ToServer> = EventQueue::default();
    let server_tx = server_msg_queue.sender().clone();

    // start connection to server
    let socket = TcpStream::connect(server_addr.clone())
        .map_ok(utils::frame_socket::<ToClient, ToServer>)
        .await
        .unwrap_or_else(|_| panic!("Could not connect to the server {}", server_addr));

    let connection_loop = async move {
        let (mut server_to_client, mut client_to_server) = socket;

        // send heartbeats every couple seconds otherwise server will disconnect
        let mut heartbeat = tokio::time::interval(Duration::from_secs(2));
        loop {
            tokio::select! {
                _ = heartbeat.tick() => server_msg_queue.sender().send(ToServer::Heartbeat),

                Some(to_server_msg) = server_msg_queue.recv_async() => {
                    if let ToServer::Leave = to_server_msg{
                        let _ = client_to_server.send(to_server_msg).await;
                        break;
                    } else if client_to_server.send(to_server_msg).await.is_err() {
                        break ;
                    }
                }

                Some(server_msg) = server_to_client.next() => {
                    match server_msg {
                        Ok(ToClient::Disconnect(reason))  => {
                            app_tx.send(Event::ServerMessage(ToClient::Disconnect(reason)));
                            break
                        },

                        Ok(msg) => app_tx.send(Event::ServerMessage(msg)),

                        _ => break,
                    };
                }

                else => break ,
            };
        }
        app_tx.send_with_urgency(Event::ConnectionDropped);
    };

    // spawn connection loop
    tokio::spawn(connection_loop);

    server_tx
}

use serde::{Deserialize, Serialize};

use crate::game::{GameInfo, Player};

pub const HEARTBEAT_INTERVAL_SECS: u64 = 5;

/// Client -> Server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToServer {
    Heartbeat,
    PassBall(Player),
    Leave,
}

/// Server -> Client
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToClient {
    InitialState(Player, GameState),
    PlayerJoin(Player),
    PlayerLeave(Player),
    PassBall(Player, WhoPassed),
    Disconnect(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WhoPassed {
    Player,
    PlayerWithBallLeft,
    PlayerStumbledUponBall,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub info: GameInfo,
    pub players: Vec<Player>,
}

use std::fmt::{Debug, Display};

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct Player(pub usize);

impl Player {
    pub fn as_ref_str(&self, other: &Player) -> String {
        if self.0 == other.0 {
            "themselves".into()
        } else {
            other.to_string()
        }
    }
}
impl Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("#{}").field(&self.0).finish()
    }
}
impl Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "player #{}", self.0)
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GameInfo {
    pub player_with_ball: Option<Player>,
}

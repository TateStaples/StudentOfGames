use std::fmt::Debug;
use std::hash::Hash;

// ---------- Tune-ables ----------
pub const SOLVE_TIME_SECS: u64 = 1;
pub const MIN_INFO_SIZE: usize = 64;
pub const MAX_SUPPORT: usize = 3;

// ---------- Basic types ----------
pub type Reward = f64;
pub type Probability = f64;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Player { P1, P2, Random }

impl Player {
    #[inline] pub fn other(self) -> Player {
        match self { Player::P1 => Player::P2, Player::P2 => Player::P1, _ => self }
    }
    pub fn best_value(self) -> Reward {
        match self { Player::P1 => 1.0, Player::P2 => -1.0, _=> panic!("Random player has no best value") }
    }
    pub fn worst_value(self) -> Reward {
        match self { Player::P1 => -1.0, Player::P2 => 1.0, _ => panic!("Random player has no worst value") }
    }
}

// ---------- Traits the game must provide ----------
pub trait ActionI: Clone + Eq + Hash + Debug {}
impl<T: Clone + Eq + Hash + Debug> ActionI for T {}

pub trait TraceI: Clone + Eq + Hash + Debug + Default + PartialOrd {
    fn player(&self) -> Player;
}

pub trait Game: Sized + Clone {
    type State: Clone;
    type Action: ActionI;
    type Observation: Clone;
    type Trace: TraceI;

    // Encode/decode world state
    fn encode(&self) -> Self::State;
    fn decode(state: &Self::State) -> Self;
    fn new() -> Self;

    // Public trace + perspective helpers
    fn trace(&self, player: Player) -> Self::Trace;
    fn perspective(&self, trace: Self::Trace) -> Player;

    // Local dynamics
    fn active_player(&self) -> Player;
    fn available_actions(&self) -> Vec<Self::Action>;
    fn observation(&self, player: Player) -> Self::Observation;
    fn play(&self, action: &Self::Action) -> Self;
    fn is_over(&self) -> bool;
    fn evaluate(&self) -> Reward; // a quick static eval

    // Pluggable sampler to seed subgames
    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item = Self>;
}

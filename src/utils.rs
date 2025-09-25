use std::fmt::{Debug};
use std::hash::Hash;
use crate::history::History;

// ---------- Tune-ables ---------- // 
pub const SOLVE_TIME_SECS: f64 = 3.0;
pub const MIN_INFO_SIZE: usize = 64;
pub const MAX_SUPPORT: usize = 3;

// ---------- Basic types ---------- //
pub type Reward = f64;
pub type Counterfactual = Reward;  // Syntactically different but Semantically same as Reward
pub type Probability = f64;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Player { P1, P2, Chance }

impl Player {
    #[inline] pub fn other(self) -> Player {
        match self { Player::P1 => Player::P2, Player::P2 => Player::P1, _ => self }
    }
    pub fn best_value(self) -> Reward {
        match self { Player::P1 => 1.0, Player::P2 => -1.0, _=> 0.0 }
    }
    pub fn worst_value(self) -> Reward {
        match self { Player::P1 => -1.0, Player::P2 => 1.0, _ => panic!("Random player has no worst value") }
    }
}

// ---------- Traits the game must provide ----------
pub trait ActionI: Clone + Eq + Hash + Debug {}
impl<T: Clone + Eq + Hash + Debug> ActionI for T {}

pub trait TraceI: Clone + Eq + Hash + Debug + Default + PartialOrd  {
    // fn player(&self) -> Player;
}


pub trait Game: Sized + Clone + Debug + Hash {
    /// Optional compressed representatino of game state for recovery
    type State: Clone;
    /// The actions that could possibly be taken
    type Action: ActionI;
    /// Represent a given player's view of what has happened
    type Trace: TraceI;

    // Encode/decode world state
    fn encode(&self) -> Self::State;
    fn decode(state: &Self::State) -> Self;
    fn new() -> Self;

    // Public trace + perspective helpers
    /// Gets the summary of what a given player knows in this board state
    fn trace(&self, player: Player) -> Self::Trace;
    // Local dynamics
    /// The player whose turn it is
    fn active_player(&self) -> Player;
    /// What actions the active_player can take
    fn available_actions(&self) -> Vec<Self::Action>;
    /// Create a new copy of the game after this specified action is taken
    fn play(&self, action: &Self::Action) -> Self;
    /// Check if the game is over
    fn is_over(&self) -> bool;
    /// Heuristic Evaluataion of the current position (+ good for P1, - for P2). Must be implemented at terminal
    fn evaluate(&self) -> Reward; // a quick static eval

    // Pluggable sampler to seed subgames
    /// Given what a player has seen, what possible positions could they be in
    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item = Self>;
    
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let hash_item = self.identifier();
        hash_item.hash(state);
    }
    
    fn identifier(&self) -> (Self::Trace, Self::Trace) {
        let active = self.active_player();
        let hero = if active == Player::Chance {Player::P1} else { active };
        let villan = hero.other();
        let hero_trace = self.trace(hero);
        let villan_trace = self.trace(villan);
        (hero_trace, villan_trace)
    }
}

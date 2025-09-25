use std::fmt::{Debug};
use std::hash::Hash;

// ---------- Tune-ables ---------- // 
pub const SOLVE_TIME_SECS: f64 = 30.0;  // How long the bot is allowed to spend developing strat
pub const MIN_INFO_SIZE: usize = 64;  // What root history size the bot should sample to
pub const MAX_SUPPORT: usize = 3;  // (not currently used) number of top actions to consider

// ---------- Basic types (renamed for pretty) ---------- //
pub type Reward = f64;
pub type Counterfactual = Reward;  // Syntactically different but Semantically same as Reward
pub type Probability = f64;

/// We only look at two player games (for provable convergence)
/// at all points a player is active or the game will do something random
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
/// Properties we want all game actions to have
pub trait ActionI: Clone + Eq + Hash + Debug {}  // see Game trait fo rmore details
impl<T: Clone + Eq + Hash + Debug> ActionI for T {}  // for some reason rust wants this
/// Properities we want to all Traces to have
pub trait TraceI: Clone + Eq + Hash + Debug + Default + PartialOrd  {} // see Game trait fo rmore details


pub trait Game: Sized + Clone + Debug + Hash {
    /// Optional compressed representatino of game state for recovery
    type State: Clone;
    /// The actions that could possibly be taken
    type Action: ActionI;
    /// Represent a given player's view of what has happened
    type Trace: TraceI;

    /// Requires a constructor
    fn new() -> Self;

    // Encode/decode world state
    /// Convert between full game and compressed state (default to State = Self)
    fn encode(&self) -> Self::State;
    /// Convert between compressed state and full game (default to State = Self)
    fn decode(state: &Self::State) -> Self;

    /// Gets the summary of what a given player knows in this board state
    fn trace(&self, player: Player) -> Self::Trace;
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
    /// Given what a player has seen, what possible positions could they be in
    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item = Self>;
    /// I think this was useful for checking if we had already added this game to tree (might be deprecated)
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let hash_item = self.identifier();
        hash_item.hash(state);
    }
    /// We uniquely identify a gamestate by what all the players know (hidden state should be superpositions until Chance nodes)
    fn identifier(&self) -> (Self::Trace, Self::Trace) {
        let active = self.active_player();
        let hero = if active == Player::Chance {Player::P1} else { active };
        let villan = hero.other();
        let hero_trace = self.trace(hero);
        let villan_trace = self.trace(villan);
        (hero_trace, villan_trace)
    }
}

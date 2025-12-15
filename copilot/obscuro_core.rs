// obscuro_core.rs - Core data structures and traits for Obscuro implementation
//
// This file defines the fundamental types and traits used throughout the Obscuro algorithm.

use std::collections::HashMap;
use std::hash::Hash;

// ============================================================================
// Basic Types
// ============================================================================

/// Reward value (utility) for a player
pub type Reward = f64;

/// Probability value [0.0, 1.0]
pub type Probability = f64;

/// Player identifier
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Player {
    /// Player 1 (maximizing player)
    P1,
    /// Player 2 (minimizing player)
    P2,
    /// Chance player (nature/randomness)
    Chance,
}

impl Player {
    /// Get the opponent player
    pub fn other(&self) -> Player {
        match self {
            Player::P1 => Player::P2,
            Player::P2 => Player::P1,
            Player::Chance => panic!("Chance player has no opponent"),
        }
    }

    /// Get best possible value for this player
    /// Used for initializing alternative values in resolvers
    pub fn best_value(&self) -> Reward {
        match self {
            Player::P1 => f64::INFINITY,
            Player::P2 => f64::NEG_INFINITY,
            Player::Chance => 0.0,
        }
    }

    /// Get worst possible value for this player
    pub fn worst_value(&self) -> Reward {
        match self {
            Player::P1 => f64::NEG_INFINITY,
            Player::P2 => f64::INFINITY,
            Player::Chance => 0.0,
        }
    }
}

// ============================================================================
// Game Trait
// ============================================================================

/// Trait that must be implemented by any game to use Obscuro
///
/// This defines the interface between the game logic and the search algorithm.
pub trait Game: Clone + Sized {
    /// The complete state of the game (internal representation)
    type State: Clone;

    /// An action that can be taken in the game
    type Action: Clone + Eq + Hash;

    /// The observation a player sees (may be partial)
    type Observation: Clone;

    /// The trace/history of observations for a player (defines their information set)
    type Trace: Clone + Eq + Hash + PartialOrd + Default;

    /// Create a new game in the starting position
    fn new() -> Self;

    /// Get the current player to act
    fn active_player(&self) -> Player;

    /// Check if the game is over
    fn is_terminal(&self) -> bool;

    /// Get the payoff at a terminal state (from Player 1's perspective)
    fn payoff(&self) -> Reward;

    /// Get all legal actions for the current player
    fn legal_actions(&self) -> Vec<Self::Action>;

    /// Apply an action and return the new game state
    fn apply_action(&self, action: &Self::Action) -> Self;

    /// Get the observation for a specific player
    fn get_observation(&self, player: Player) -> Self::Observation;

    /// Get the trace (information set identifier) for a specific player
    fn trace(&self, player: Player) -> Self::Trace;

    /// Get a unique identifier for this game state (for deduplication)
    fn identifier(&self) -> u64;

    /// Evaluate the position (heuristic value from Player 1's perspective)
    /// Used as alternative value in safe resolving
    fn evaluate(&self) -> Reward;

    /// Sample possible game positions consistent with given trace
    /// Returns an iterator over possible positions
    fn sample_positions(trace: Self::Trace) -> Box<dyn Iterator<Item = Self>>;

    /// Encode current state
    fn encode(&self) -> Self::State;

    /// Decode state back to game
    fn decode(state: &Self::State) -> Self;
}

// ============================================================================
// Policy - Represents strategy at an information set
// ============================================================================

/// Policy stores the strategy and learning data for an information set
#[derive(Debug, Clone)]
pub struct Policy<A: Clone + Eq + Hash> {
    /// The player who acts at this information set
    pub player: Player,

    /// Available actions
    pub actions: Vec<A>,

    /// Cumulative regrets for each action (for CFR+)
    pub cumulative_regrets: HashMap<A, f64>,

    /// Cumulative strategy weights (for computing average strategy)
    pub cumulative_strategy: HashMap<A, f64>,

    /// Number of times this policy has been updated
    pub update_count: usize,

    /// Number of times each action has been explored (for tree expansion)
    pub exploration_count: HashMap<A, usize>,
}

impl<A: Clone + Eq + Hash> Policy<A> {
    /// Create a new uniform policy over given actions
    pub fn new(actions: Vec<A>, player: Player) -> Self {
        let mut cumulative_regrets = HashMap::new();
        let mut cumulative_strategy = HashMap::new();
        let mut exploration_count = HashMap::new();

        for action in &actions {
            cumulative_regrets.insert(action.clone(), 0.0);
            cumulative_strategy.insert(action.clone(), 0.0);
            exploration_count.insert(action.clone(), 0);
        }

        Policy {
            player,
            actions,
            cumulative_regrets,
            cumulative_strategy,
            update_count: 0,
            exploration_count,
        }
    }

    /// Get the current strategy (regret-matching)
    pub fn current_strategy(&self) -> HashMap<A, Probability> {
        if self.actions.is_empty() {
            return HashMap::new();
        }

        // Regret matching+: only use positive regrets
        let positive_regrets: HashMap<A, f64> = self
            .cumulative_regrets
            .iter()
            .map(|(a, &r)| (a.clone(), r.max(0.0)))
            .collect();

        let total_positive_regret: f64 = positive_regrets.values().sum();

        if total_positive_regret > 0.0 {
            // Use regret-matching
            positive_regrets
                .into_iter()
                .map(|(a, r)| (a, r / total_positive_regret))
                .collect()
        } else {
            // Uniform distribution if no positive regrets
            let prob = 1.0 / self.actions.len() as f64;
            self.actions.iter().map(|a| (a.clone(), prob)).collect()
        }
    }

    /// Get the average strategy (for Nash approximation)
    pub fn average_strategy(&self) -> HashMap<A, Probability> {
        if self.actions.is_empty() {
            return HashMap::new();
        }

        let total_weight: f64 = self.cumulative_strategy.values().sum();

        if total_weight > 0.0 {
            self.cumulative_strategy
                .iter()
                .map(|(a, &w)| (a.clone(), w / total_weight))
                .collect()
        } else {
            // Uniform if no strategy weight accumulated
            let prob = 1.0 / self.actions.len() as f64;
            self.actions.iter().map(|a| (a.clone(), prob)).collect()
        }
    }

    /// Add counterfactual value for an action
    pub fn add_counterfactual(&mut self, action: &A, value: Reward, reach_prob: Probability) {
        if let Some(regret) = self.cumulative_regrets.get_mut(action) {
            *regret += reach_prob * value;
        }
    }

    /// Update the policy after CFR iteration
    pub fn update(&mut self, iteration: usize) {
        self.update_count = iteration;

        // Add current strategy to cumulative strategy
        let current = self.current_strategy();
        for (action, prob) in current {
            *self.cumulative_strategy.entry(action).or_insert(0.0) += prob;
        }
    }

    /// Record that an action was explored during tree expansion
    pub fn record_exploration(&mut self, action: &A) {
        *self.exploration_count.entry(action.clone()).or_insert(0) += 1;
    }

    /// Get action to explore (using UCB-like selection)
    pub fn select_exploration(&self) -> A {
        if self.actions.is_empty() {
            panic!("No actions available");
        }

        // Simple UCB: prefer less-explored actions
        let total_explorations: usize = self.exploration_count.values().sum();
        let exploration_bonus = (total_explorations as f64).ln().max(1.0);

        let current = self.current_strategy();

        self.actions
            .iter()
            .max_by(|a, b| {
                let explore_a = *self.exploration_count.get(a).unwrap_or(&0);
                let explore_b = *self.exploration_count.get(b).unwrap_or(&0);
                let value_a = current.get(a).unwrap_or(&0.0);
                let value_b = current.get(b).unwrap_or(&0.0);

                let score_a = value_a + exploration_bonus / (1.0 + explore_a as f64);
                let score_b = value_b + exploration_bonus / (1.0 + explore_b as f64);

                score_a.partial_cmp(&score_b).unwrap()
            })
            .unwrap()
            .clone()
    }

    /// Get best action according to current strategy (for exploitation)
    pub fn best_action(&self) -> A {
        if self.actions.is_empty() {
            panic!("No actions available");
        }

        let strategy = self.current_strategy();
        self.actions
            .iter()
            .max_by(|a, b| {
                let pa = strategy.get(a).unwrap_or(&0.0);
                let pb = strategy.get(b).unwrap_or(&0.0);
                pa.partial_cmp(pb).unwrap()
            })
            .unwrap()
            .clone()
    }

    /// Get expected value according to current strategy
    pub fn expected_value(&self, values: &HashMap<A, Reward>) -> Reward {
        let strategy = self.current_strategy();
        self.actions
            .iter()
            .map(|a| strategy.get(a).unwrap_or(&0.0) * values.get(a).unwrap_or(&0.0))
            .sum()
    }
}

// ============================================================================
// History - Represents a node in the game tree
// ============================================================================

/// Represents a node in the game tree with different expansion states
#[derive(Clone)]
pub enum History<G: Game> {
    /// Terminal node (game over)
    Terminal {
        game: G,
        payoff: Reward,
        reach_probs: HashMap<Player, Probability>,
    },

    /// Visited but not yet expanded
    Visited {
        game: G,
        payoff: Reward,
        reach_probs: HashMap<Player, Probability>,
    },

    /// Expanded node with children
    Expanded {
        game: G,
        player: Player,
        children: Vec<(G::Action, History<G>)>,
        reach_probs: HashMap<Player, Probability>,
    },
}

impl<G: Game> History<G> {
    /// Create a new history node from a game state
    pub fn new(game: G) -> Self {
        if game.is_terminal() {
            History::Terminal {
                payoff: game.payoff(),
                game,
                reach_probs: HashMap::new(),
            }
        } else {
            History::Visited {
                payoff: game.evaluate(),
                game,
                reach_probs: HashMap::new(),
            }
        }
    }

    /// Get the game state
    pub fn game(&self) -> &G {
        match self {
            History::Terminal { game, .. } => game,
            History::Visited { game, .. } => game,
            History::Expanded { game, .. } => game,
        }
    }

    /// Get the trace for a specific player
    pub fn trace(&self, player: Player) -> G::Trace {
        self.game().trace(player)
    }

    /// Get the identifier of this position
    pub fn identifier(&self) -> u64 {
        self.game().identifier()
    }

    /// Get net reach probability (product of all players except self)
    pub fn net_reach_prob(&self) -> Probability {
        match self {
            History::Terminal { reach_probs, .. }
            | History::Visited { reach_probs, .. }
            | History::Expanded { reach_probs, .. } => {
                reach_probs.values().product::<Probability>().max(1e-12)
            }
        }
    }

    /// Set reach probability for a player
    pub fn set_reach_prob(&mut self, player: Player, prob: Probability) {
        let reach_probs = match self {
            History::Terminal { reach_probs, .. } => reach_probs,
            History::Visited { reach_probs, .. } => reach_probs,
            History::Expanded { reach_probs, .. } => reach_probs,
        };
        reach_probs.insert(player, prob);
    }

    /// Get the current player
    pub fn active_player(&self) -> Player {
        match self {
            History::Terminal { .. } => Player::Chance,
            History::Visited { game, .. } => game.active_player(),
            History::Expanded { player, .. } => *player,
        }
    }

    /// Count nodes in this subtree
    pub fn size(&self) -> usize {
        match self {
            History::Terminal { .. } | History::Visited { .. } => 1,
            History::Expanded { children, .. } => {
                1 + children.iter().map(|(_, h)| h.size()).sum::<usize>()
            }
        }
    }
}

// ============================================================================
// InfoSet - Information set tracking
// ============================================================================

/// Information set data structure
#[derive(Clone)]
pub struct InfoSet<A: Clone + Eq + Hash, T: Clone + Eq + Hash> {
    /// The trace identifying this information set
    pub trace: T,

    /// The player who acts at this information set
    pub player: Player,

    /// The policy (strategy) at this information set
    pub policy: Policy<A>,
}

impl<A: Clone + Eq + Hash, T: Clone + Eq + Hash> InfoSet<A, T> {
    /// Create a new information set
    pub fn new(trace: T, actions: Vec<A>, player: Player) -> Self {
        InfoSet {
            trace,
            player,
            policy: Policy::new(actions, player),
        }
    }
}

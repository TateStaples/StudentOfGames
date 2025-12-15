// obscuro_algorithm.rs - Main Obscuro algorithm implementation
//
// This module ties together all the components to implement the complete
// Obscuro search algorithm for imperfect-information games.

use crate::cfr_plus::*;
use crate::obscuro_core::*;
use crate::safe_resolving::*;
use crate::subgame_solving::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

// ============================================================================
// Configuration
// ============================================================================

/// Configuration parameters for Obscuro
#[derive(Debug, Clone)]
pub struct ObscuroConfig {
    /// Time budget per move (in seconds)
    pub time_per_move: f64,

    /// Minimum number of positions to sample at subgame root
    pub min_positions: usize,

    /// Depth of k-cover (knowledge reasoning depth)
    pub k_depth: usize,

    /// Number of CFR iterations per expansion step
    pub cfr_iterations_per_expansion: usize,

    /// Maximum tree size (for memory management)
    pub max_tree_size: usize,
}

impl Default for ObscuroConfig {
    fn default() -> Self {
        ObscuroConfig {
            time_per_move: 5.0,
            min_positions: 64,
            k_depth: 3,
            cfr_iterations_per_expansion: 10,
            max_tree_size: 1_000_000,
        }
    }
}

// ============================================================================
// Main Obscuro Structure
// ============================================================================

/// The Obscuro search algorithm
pub struct Obscuro<G: Game> {
    /// Configuration
    config: ObscuroConfig,

    /// Information sets indexed by trace
    info_sets: HashMap<G::Trace, Rc<RefCell<InfoSet<G::Action, G::Trace>>>>,

    /// Current subgame root
    subgame_root: Option<SubgameRoot<G>>,

    /// Total number of iterations
    total_iterations: usize,

    /// Saved game tree from previous moves
    saved_tree: Vec<History<G>>,

    /// Expected value of current position
    expected_value: Reward,
}

impl<G: Game> Obscuro<G> {
    /// Create a new Obscuro instance with default configuration
    pub fn new() -> Self {
        Self::with_config(ObscuroConfig::default())
    }

    /// Create a new Obscuro instance with custom configuration
    pub fn with_config(config: ObscuroConfig) -> Self {
        Obscuro {
            config,
            info_sets: HashMap::new(),
            subgame_root: None,
            total_iterations: 0,
            saved_tree: vec![History::new(G::new())],
            expected_value: 0.0,
        }
    }

    /// Main entry point: compute strategy for a position and return chosen action
    pub fn make_move(&mut self, observation: G::Trace, player: Player) -> G::Action {
        // Step 1: Study the position (build and solve subgame)
        self.study_position(observation.clone(), player);

        // Step 2: Select action from computed strategy
        self.select_action(observation, player)
    }

    /// Compute strategy for a position (Steps 1-4 of the algorithm)
    pub fn study_position(&mut self, observation: G::Trace, player: Player) {
        let start_time = Instant::now();
        let time_budget = Duration::from_secs_f64(self.config.time_per_move);

        println!("=== Studying position for player {:?} ===", player);

        // Step 1: Construct subgame
        self.construct_subgame(observation.clone(), player);

        if let Some(ref subgame) = self.subgame_root {
            println!("Subgame constructed: {} resolvers, {} total histories, tree size: {}",
                     subgame.resolvers.len(),
                     subgame.total_histories(),
                     subgame.tree_size());
        }

        // Steps 2-4: Iteratively expand and solve until time budget exhausted
        let mut expansion_count = 0;
        let mut cfr_count = 0;

        while start_time.elapsed() < time_budget {
            // Step 3: Expand the tree
            self.expansion_step(player);
            expansion_count += 1;

            // Step 2: Run CFR iterations
            for _ in 0..self.config.cfr_iterations_per_expansion {
                self.cfr_step(player);
                cfr_count += 1;
            }

            // Check tree size
            if let Some(ref subgame) = self.subgame_root {
                if subgame.tree_size() > self.config.max_tree_size {
                    println!("Warning: Tree size exceeded limit, stopping expansion");
                    break;
                }
            }
        }

        println!("Completed: {} expansions, {} CFR iterations, {:.2}s elapsed",
                 expansion_count, cfr_count, start_time.elapsed().as_secs_f64());

        if let Some(ref subgame) = self.subgame_root {
            println!("Final tree size: {}", subgame.tree_size());
        }
    }

    /// Construct subgame using KLUSS
    fn construct_subgame(&mut self, observation: G::Trace, player: Player) {
        // Extract and filter old tree
        let old_tree = std::mem::take(&mut self.saved_tree);
        let filtered_tree = prune_old_tree(old_tree, observation.clone(), player);

        // Build subgame using k-cover
        let positions = construct_subgame(
            filtered_tree,
            observation,
            player,
            self.config.min_positions,
            self.config.k_depth,
        );

        // Create subgame root with resolver gadgets
        self.subgame_root = Some(SubgameRoot::new(positions, player));
    }

    /// Perform one expansion step (GT-CFR tree growth)
    fn expansion_step(&mut self, player: Player) {
        // Get the indices without complex borrowing
        let indices = if let Some(ref subgame) = self.subgame_root {
            subgame.sample_history()
        } else {
            None
        };

        if let Some((resolver_idx, history_idx)) = indices {
            // Temporarily take ownership of subgame_root
            if let Some(mut subgame) = self.subgame_root.take() {
                let history = subgame.get_history_mut(resolver_idx, history_idx);
                
                // Expand the history
                Self::expand_history_static(history, player, &mut self.info_sets);
                
                // Restore subgame_root
                self.subgame_root = Some(subgame);
            }
        }
    }

    /// Static version of expand_history that doesn't borrow self
    fn expand_history_static(
        mut history: &mut History<G>,
        target_player: Player,
        info_sets: &mut HashMap<G::Trace, Rc<RefCell<InfoSet<G::Action, G::Trace>>>>,
    ) {
        // Navigate to leaf
        loop {
            match history {
                History::Expanded { game, player, children, .. } => {
                    let trace = game.trace(*player);
                    let game_clone = game.clone();
                    let player_clone = *player;

                    // Get or create info set
                    let info_set = info_sets
                        .entry(trace.clone())
                        .or_insert_with(|| {
                            let actions = game_clone.legal_actions();
                            Rc::new(RefCell::new(InfoSet::new(trace, actions, player_clone)))
                        })
                        .clone();

                    let mut info = info_set.borrow_mut();

                    // Select action
                    let action = if player_clone == target_player {
                        info.policy.select_exploration()
                    } else {
                        info.policy.best_action()
                    };

                    info.policy.record_exploration(&action);
                    drop(info);

                    // Find child and continue
                    let mut found_child = None;
                    for (a, child) in children.iter_mut() {
                        if a == &action {
                            found_child = Some(child);
                            break;
                        }
                    }

                    match found_child {
                        Some(child) => history = child,
                        None => return,
                    }
                }
                
                History::Visited { game, .. } => {
                    // Expand this node
                    let game_clone = game.clone();
                    let player = game_clone.active_player();
                    let actions = game_clone.legal_actions();
                    let trace = game_clone.trace(player);

                    // Create info set
                    let _info_set = info_sets
                        .entry(trace.clone())
                        .or_insert_with(|| {
                            Rc::new(RefCell::new(InfoSet::new(trace, actions.clone(), player)))
                        })
                        .clone();

                    // Create children
                    let children: Vec<(G::Action, History<G>)> = actions
                        .into_iter()
                        .map(|action| {
                            let new_game = game_clone.apply_action(&action);
                            (action, History::new(new_game))
                        })
                        .collect();

                    // Replace with Expanded
                    *history = History::Expanded {
                        game: game_clone,
                        player,
                        children,
                        reach_probs: HashMap::new(),
                    };
                    return;
                }
                
                History::Terminal { .. } => {
                    return;
                }
            }
        }
    }

    /// Perform one CFR iteration
    fn cfr_step(&mut self, player: Player) {
        if let Some(ref mut subgame) = self.subgame_root {
            self.total_iterations += 1;

            // Run CFR for both players
            let _value_p1 = cfr_iteration(subgame, Player::P1, &mut self.info_sets, self.total_iterations);
            let _value_p2 = cfr_iteration(subgame, Player::P2, &mut self.info_sets, self.total_iterations);

            // Update expected value
            self.expected_value = if player == Player::P1 {
                _value_p1
            } else {
                -_value_p2
            };
        }
    }

    /// Select action to play (Step 5)
    fn select_action(&self, observation: G::Trace, _player: Player) -> G::Action {
        // Get info set for current observation
        if let Some(info_set) = self.info_sets.get(&observation) {
            let info = info_set.borrow();

            // Use average strategy for final decision
            let avg_strategy = info.policy.average_strategy();

            // Select action (could be probabilistic or deterministic)
            // For now, return best action from average strategy
            return info
                .policy
                .actions
                .iter()
                .max_by(|a, b| {
                    let pa = avg_strategy.get(a).unwrap_or(&0.0);
                    let pb = avg_strategy.get(b).unwrap_or(&0.0);
                    pa.partial_cmp(pb).unwrap()
                })
                .unwrap()
                .clone();
        }

        // Fallback: create a new game and pick first legal action
        let game = G::new();
        let actions = game.legal_actions();
        if !actions.is_empty() {
            actions[0].clone()
        } else {
            panic!("No legal actions available");
        }
    }

    /// Get the current policy for an information set
    pub fn get_policy(&self, trace: G::Trace) -> Option<HashMap<G::Action, Probability>> {
        self.info_sets
            .get(&trace)
            .map(|info_set| info_set.borrow().policy.average_strategy())
    }

    /// Get expected value of current position
    pub fn get_expected_value(&self) -> Reward {
        self.expected_value
    }

    /// Get statistics about the search
    pub fn get_stats(&self) -> SearchStats {
        let tree_size = self
            .subgame_root
            .as_ref()
            .map(|s| s.tree_size())
            .unwrap_or(0);

        SearchStats {
            total_iterations: self.total_iterations,
            info_sets_count: self.info_sets.len(),
            tree_size,
            expected_value: self.expected_value,
        }
    }

    /// Save current subgame tree for next move
    pub fn save_tree(&mut self) {
        if let Some(subgame) = &self.subgame_root {
            // Extract all histories from resolvers
            self.saved_tree = subgame
                .resolvers
                .iter()
                .flat_map(|r| r.histories.clone())
                .collect();
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Statistics about the search process
#[derive(Debug, Clone)]
pub struct SearchStats {
    pub total_iterations: usize,
    pub info_sets_count: usize,
    pub tree_size: usize,
    pub expected_value: Reward,
}

impl std::fmt::Display for SearchStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stats: {} iterations, {} infosets, tree size {}, EV: {:.3}",
            self.total_iterations, self.info_sets_count, self.tree_size, self.expected_value
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ObscuroConfig::default();
        assert!(config.time_per_move > 0.0);
        assert!(config.min_positions > 0);
        assert!(config.k_depth > 0);
    }

    #[test]
    fn test_obscuro_creation() {
        // This would need a concrete game implementation to test
        // For now, just verify the structure compiles
    }
}

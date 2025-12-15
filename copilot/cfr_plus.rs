// cfr_plus.rs - CFR+ (Counterfactual Regret Minimization Plus) implementation
//
// This module implements the CFR+ algorithm for iteratively computing
// approximate Nash equilibria in imperfect-information games.

use crate::obscuro_core::*;
use crate::safe_resolving::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// ============================================================================
// CFR+ Algorithm
// ============================================================================

/// Run one iteration of CFR+ for the specified player
///
/// This traverses the game tree and updates regrets based on counterfactual values.
pub fn cfr_iteration<G: Game>(
    subgame_root: &mut SubgameRoot<G>,
    optimizing_player: Player,
    info_sets: &mut HashMap<G::Trace, Rc<RefCell<InfoSet<G::Action, G::Trace>>>>,
    iteration: usize,
) -> Reward {
    let maxmargin_strategy = subgame_root.maxmargin_policy.current_strategy();

    let mut total_value = 0.0;

    // Iterate over each resolver gadget
    for (resolver_idx, resolver) in subgame_root.resolvers.iter_mut().enumerate() {
        let resolver_prob = maxmargin_strategy
            .get(&resolver_idx)
            .copied()
            .unwrap_or(0.0);

        // For now, assume full entry (p_enter = 1.0)
        // In full implementation, this would be resolver.prob_enter()
        let p_enter = 1.0;

        let mut enter_value = 0.0;

        // Get sampling distribution over histories
        let sampling_strategy = resolver.sampling_policy.current_strategy();

        // Process each history in this resolver
        for (history_idx, history) in resolver.histories.iter_mut().enumerate() {
            let sample_prob = sampling_strategy
                .get(&history_idx)
                .copied()
                .unwrap_or(0.0);

            if sample_prob > 0.0 {
                // Build reach probability map
                let mut reach_probs = HashMap::new();
                reach_probs.insert(Player::Chance, sample_prob);
                reach_probs.insert(optimizing_player.other(), resolver_prob * p_enter);

                // Compute counterfactual values
                let value = compute_cfr_values(
                    history,
                    optimizing_player,
                    reach_probs,
                    info_sets,
                    iteration,
                );

                enter_value += sample_prob * value;
            }
        }

        // Update resolver policy
        let skip_value = resolver.alt_value;
        resolver.update_resolver(enter_value, skip_value, resolver_prob, iteration);

        // Compute expected value for this resolver
        let resolver_value = (1.0 - p_enter) * skip_value + p_enter * enter_value;

        // Update maxmargin policy
        subgame_root
            .maxmargin_policy
            .add_counterfactual(&resolver_idx, resolver_value, 1.0);

        total_value += resolver_value * resolver.prior_probability;
    }

    // Update maxmargin policy
    subgame_root.maxmargin_policy.update(iteration);

    total_value
}

/// Compute counterfactual values recursively through the game tree
fn compute_cfr_values<G: Game>(
    history: &mut History<G>,
    optimizing_player: Player,
    reach_probs: HashMap<Player, Probability>,
    info_sets: &mut HashMap<G::Trace, Rc<RefCell<InfoSet<G::Action, G::Trace>>>>,
    iteration: usize,
) -> Reward {
    match history {
        History::Terminal { payoff, .. } => {
            // Terminal node: return payoff
            *payoff
        }

        History::Visited { payoff, reach_probs: node_reach, .. } => {
            // Visited but not expanded: return heuristic value
            *node_reach = reach_probs;
            *payoff
        }

        History::Expanded {
            game,
            player,
            children,
            reach_probs: node_reach,
        } => {
            *node_reach = reach_probs.clone();

            let trace = game.trace(*player);

            // Get or create info set
            let info_set = info_sets
                .entry(trace.clone())
                .or_insert_with(|| {
                    let actions = game.legal_actions();
                    Rc::new(RefCell::new(InfoSet::new(trace.clone(), actions, *player)))
                })
                .clone();

            let mut info = info_set.borrow_mut();

            // Get current strategy
            let strategy = info.policy.current_strategy();

            // Compute net reach probability (all players except current)
            let net_reach: Probability = reach_probs
                .iter()
                .filter(|(p, _)| **p != *player)
                .map(|(_, prob)| *prob)
                .product();

            let mut expected_value = 0.0;

            // Compute value for each action
            for (action, child) in children.iter_mut() {
                let action_prob = strategy.get(action).copied().unwrap_or(0.0);

                // Only explore if optimizing player plays here or opponent gives positive prob
                if *player == optimizing_player || action_prob > 0.0 {
                    // Update reach probabilities for this action
                    let mut child_reach = reach_probs.clone();
                    child_reach
                        .entry(*player)
                        .and_modify(|p| *p *= action_prob)
                        .or_insert(action_prob);

                    // Recursively compute value
                    let value = compute_cfr_values(
                        child,
                        optimizing_player,
                        child_reach,
                        info_sets,
                        iteration,
                    );

                    // Add to expected value
                    expected_value += action_prob * value;

                    // Update regrets if this is optimizing player's node
                    if *player == optimizing_player {
                        info.policy.add_counterfactual(action, value, net_reach);
                    }
                }
            }

            // Update policy
            info.policy.update(iteration);

            expected_value
        }
    }
}

/// Apply policy updates after CFR iteration
///
/// This is called after computing counterfactual values to update all policies
pub fn apply_policy_updates<G: Game>(
    history: &mut History<G>,
    info_sets: &mut HashMap<G::Trace, Rc<RefCell<InfoSet<G::Action, G::Trace>>>>,
    iteration: usize,
) {
    match history {
        History::Terminal { .. } | History::Visited { .. } => {
            // Nothing to update
        }

        History::Expanded {
            game,
            player,
            children,
            ..
        } => {
            let trace = game.trace(*player);

            // Update this node's policy
            if let Some(info_set) = info_sets.get(&trace) {
                info_set.borrow_mut().policy.update(iteration);
            }

            // Recursively update children
            for (_, child) in children.iter_mut() {
                apply_policy_updates(child, info_sets, iteration);
            }
        }
    }
}

// ============================================================================
// Predictive CFR+ Extensions
// ============================================================================

/// Predictive CFR+ uses predictions of future regrets to accelerate convergence
///
/// This is a placeholder for the full PCFR+ algorithm which would include:
/// - Predicting future regrets based on recent history
/// - Alternating between regret matching and best response
/// - Linear averaging of strategies
#[allow(dead_code)]
pub struct PredictiveCFR {
    /// History of regrets for prediction
    regret_history: Vec<HashMap<String, f64>>,

    /// Prediction weights
    prediction_weight: f64,
}

impl PredictiveCFR {
    pub fn new() -> Self {
        PredictiveCFR {
            regret_history: Vec::new(),
            prediction_weight: 0.5,
        }
    }

    /// Predict future regrets (simplified version)
    pub fn predict_regrets(&self, current_regrets: &HashMap<String, f64>) -> HashMap<String, f64> {
        // In full PCFR+, this would do linear extrapolation
        // For now, just return current regrets
        current_regrets.clone()
    }
}

// ============================================================================
// CFR Metrics and Utilities
// ============================================================================

/// Compute exploitability of a strategy profile
///
/// This measures how much an opponent could exploit the current strategy
pub fn compute_exploitability<G: Game>(
    _game: &G,
    _info_sets: &HashMap<G::Trace, Rc<RefCell<InfoSet<G::Action, G::Trace>>>>,
) -> f64 {
    // Placeholder for exploitability computation
    // Would require best response calculation
    0.0
}

/// Compute average strategy from info sets
pub fn extract_average_strategy<G: Game>(
    info_sets: &HashMap<G::Trace, Rc<RefCell<InfoSet<G::Action, G::Trace>>>>,
) -> HashMap<G::Trace, HashMap<G::Action, Probability>> {
    let mut result = HashMap::new();

    for (trace, info_set) in info_sets.iter() {
        let info = info_set.borrow();
        let avg_strategy = info.policy.average_strategy();
        result.insert(trace.clone(), avg_strategy);
    }

    result
}

/// Reset regrets (used in some CFR variants)
pub fn reset_negative_regrets<G: Game>(
    info_sets: &mut HashMap<G::Trace, Rc<RefCell<InfoSet<G::Action, G::Trace>>>>,
) {
    for info_set in info_sets.values_mut() {
        let mut info = info_set.borrow_mut();
        for regret in info.policy.cumulative_regrets.values_mut() {
            if *regret < 0.0 {
                *regret = 0.0;
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_update() {
        let actions = vec![0, 1, 2];
        let mut policy = Policy::new(actions, Player::P1);

        // Add some regrets
        policy.add_counterfactual(&0, 10.0, 1.0);
        policy.add_counterfactual(&1, 5.0, 1.0);
        policy.add_counterfactual(&2, -2.0, 1.0);

        policy.update(1);

        let strategy = policy.current_strategy();

        // Action 0 should have highest probability (highest regret)
        assert!(strategy[&0] > strategy[&1]);
        assert!(strategy[&1] > strategy[&2]);
    }

    #[test]
    fn test_regret_matching_plus() {
        let actions = vec![0, 1];
        let mut policy = Policy::new(actions, Player::P1);

        // Add negative regret
        policy.add_counterfactual(&0, -10.0, 1.0);
        policy.add_counterfactual(&1, 5.0, 1.0);

        // Regret matching+ should ignore negative regrets
        let strategy = policy.current_strategy();

        // Only action 1 has positive regret
        assert!(strategy[&1] > 0.9);
    }

    #[test]
    fn test_predictive_cfr_creation() {
        let pcfr = PredictiveCFR::new();
        assert_eq!(pcfr.regret_history.len(), 0);
        assert!(pcfr.prediction_weight > 0.0);
    }
}

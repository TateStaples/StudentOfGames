// safe_resolving.rs - Safe resolving with resolver gadgets
//
// This module implements the safe resolving technique that ensures
// exploitability guarantees even when solving approximate subgames.

use crate::obscuro_core::*;
use std::collections::HashMap;

// ============================================================================
// Resolver Actions
// ============================================================================

/// Actions available at a resolver gadget
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ResolveAction {
    /// Enter the subgame (continue solving)
    Enter,
    /// Skip the subgame (take alternative value)
    Skip,
}

// ============================================================================
// Resolver Gadget
// ============================================================================

/// A resolver gadget that allows the opponent to choose between
/// entering the subgame or taking an alternative value
pub struct ResolverGadget<G: Game> {
    /// Policy for choosing between Enter/Skip
    pub resolver_policy: Policy<ResolveAction>,

    /// Alternative value if Skip is chosen
    pub alt_value: Reward,

    /// Prior probability of reaching this opponent information state
    pub prior_probability: Probability,

    /// Policy for sampling which specific history to explore
    pub sampling_policy: Policy<usize>,

    /// The actual histories at this opponent information state
    pub histories: Vec<History<G>>,

    /// The trace identifying the opponent's information state
    pub trace: G::Trace,
}

impl<G: Game> ResolverGadget<G> {
    /// Create a new resolver gadget
    pub fn new(
        histories: Vec<History<G>>,
        trace: G::Trace,
        alt_value: Reward,
        prior_probability: Probability,
        player: Player,
    ) -> Self {
        // Create sampling policy over history indices
        let indices: Vec<usize> = (0..histories.len()).collect();
        let sampling_policy = Policy::new(indices, Player::Chance);

        // Create resolver policy (Enter vs Skip)
        let actions = vec![ResolveAction::Enter, ResolveAction::Skip];
        let resolver_policy = Policy::new(actions, player.other());

        // Note: Alternative value will be used in CFR iteration as the Skip payoff
        // Initial regrets start at 0.0 as per standard CFR

        ResolverGadget {
            resolver_policy,
            alt_value,
            prior_probability,
            sampling_policy,
            histories,
            trace,
        }
    }

    /// Get the probability of entering the subgame
    pub fn prob_enter(&self) -> Probability {
        let strategy = self.resolver_policy.current_strategy();
        *strategy.get(&ResolveAction::Enter).unwrap_or(&0.0)
    }

    /// Get the probability of skipping the subgame
    pub fn prob_skip(&self) -> Probability {
        let strategy = self.resolver_policy.current_strategy();
        *strategy.get(&ResolveAction::Skip).unwrap_or(&0.0)
    }

    /// Update resolver policy with counterfactual values
    pub fn update_resolver(
        &mut self,
        enter_value: Reward,
        skip_value: Reward,
        reach_prob: Probability,
        iteration: usize,
    ) {
        self.resolver_policy
            .add_counterfactual(&ResolveAction::Enter, enter_value, reach_prob);
        self.resolver_policy
            .add_counterfactual(&ResolveAction::Skip, skip_value, reach_prob);
        self.resolver_policy.update(iteration);
    }
}

// ============================================================================
// Subgame Root
// ============================================================================

/// The root of a subgame, containing all resolver gadgets
pub struct SubgameRoot<G: Game> {
    /// Resolver gadgets for each opponent information state
    pub resolvers: Vec<ResolverGadget<G>>,

    /// Maxmargin policy over resolver gadgets
    pub maxmargin_policy: Policy<usize>,

    /// The player acting in this subgame
    pub player: Player,
}

impl<G: Game> SubgameRoot<G> {
    /// Create a new subgame root
    pub fn new(
        positions: HashMap<G::Trace, (Probability, Reward, Vec<History<G>>)>,
        player: Player,
    ) -> Self {
        let mut resolvers = Vec::new();

        // Create resolver gadget for each opponent information state
        for (trace, (prob, alt_value, histories)) in positions {
            let resolver = ResolverGadget::new(histories, trace, alt_value, prob, player);
            resolvers.push(resolver);
        }

        // Create maxmargin policy over resolver indices
        let indices: Vec<usize> = (0..resolvers.len()).collect();
        let mut maxmargin_policy = Policy::new(indices, player);

        // Initialize cumulative strategy with prior probabilities
        for (idx, resolver) in resolvers.iter().enumerate() {
            maxmargin_policy
                .cumulative_strategy
                .insert(idx, resolver.prior_probability);
        }

        SubgameRoot {
            resolvers,
            maxmargin_policy,
            player,
        }
    }

    /// Get maximum probability of any resolver entering
    pub fn max_enter_prob(&self) -> Probability {
        self.resolvers
            .iter()
            .map(|r| r.prob_enter())
            .fold(0.0, f64::max)
    }

    /// Sample a history from the subgame root
    pub fn sample_history(&self) -> Option<(usize, usize)> {
        if self.resolvers.is_empty() {
            return None;
        }

        // Sample resolver gadget
        let resolver_idx = self.maxmargin_policy.select_exploration();

        // Sample history within that gadget
        let resolver = &self.resolvers[resolver_idx];
        if resolver.histories.is_empty() {
            return None;
        }

        let history_idx = resolver.sampling_policy.select_exploration();

        Some((resolver_idx, history_idx))
    }

    /// Get mutable reference to a specific history
    pub fn get_history_mut(&mut self, resolver_idx: usize, history_idx: usize) -> &mut History<G> {
        &mut self.resolvers[resolver_idx].histories[history_idx]
    }

    /// Get total number of histories across all resolvers
    pub fn total_histories(&self) -> usize {
        self.resolvers.iter().map(|r| r.histories.len()).sum()
    }

    /// Get total tree size
    pub fn tree_size(&self) -> usize {
        self.resolvers
            .iter()
            .map(|r| r.histories.iter().map(|h| h.size()).sum::<usize>())
            .sum()
    }
}

// ============================================================================
// Safe Resolving Utilities
// ============================================================================

/// Compute safe resolving strategy that accounts for opponent's enter/skip choice
pub fn compute_safe_strategy<G: Game>(
    subgame_root: &mut SubgameRoot<G>,
    iteration: usize,
) -> HashMap<G::Trace, Policy<G::Action>> {
    let p_max = subgame_root.max_enter_prob();

    // Update maxmargin policy based on resolver choices
    for (idx, resolver) in subgame_root.resolvers.iter_mut().enumerate() {
        let p_maxmargin = subgame_root
            .maxmargin_policy
            .current_strategy()
            .get(&idx)
            .copied()
            .unwrap_or(0.0);

        let p_resolve = resolver.prob_enter();

        // Compute reach probability considering both maxmargin and resolver
        let reach_prob = p_max * resolver.prior_probability * p_resolve
            + (1.0 - p_max) * p_maxmargin;

        // Update maxmargin policy
        subgame_root
            .maxmargin_policy
            .add_counterfactual(&idx, reach_prob, 1.0);
    }

    subgame_root.maxmargin_policy.update(iteration);

    // Extract strategies from the subgame
    // This would normally collect all the policies from expanded nodes
    // For now, return empty map (would be populated during CFR)
    HashMap::new()
}

/// Initialize resolver values based on previous solution or evaluation
pub fn initialize_resolver_values<G: Game>(
    histories: &[History<G>],
    player: Player,
) -> Reward {
    if histories.is_empty() {
        return player.worst_value();
    }

    // Average the evaluation across all histories
    let total: Reward = histories
        .iter()
        .map(|h| match h {
            History::Terminal { payoff, .. } => *payoff,
            History::Visited { payoff, .. } => *payoff,
            History::Expanded { game, .. } => game.evaluate(),
        })
        .sum();

    total / histories.len() as f64
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_policy() {
        let actions = vec![ResolveAction::Enter, ResolveAction::Skip];
        let mut policy = Policy::new(actions, Player::P1);

        // Initially should be uniform
        let strategy = policy.current_strategy();
        assert!((strategy[&ResolveAction::Enter] - 0.5).abs() < 0.01);

        // Add regret for Enter
        policy.add_counterfactual(&ResolveAction::Enter, 10.0, 1.0);
        policy.update(1);

        // Should now prefer Enter
        let strategy = policy.current_strategy();
        assert!(strategy[&ResolveAction::Enter] > 0.5);
    }

    #[test]
    fn test_resolve_action_enum() {
        let enter = ResolveAction::Enter;
        let skip = ResolveAction::Skip;

        assert_ne!(enter, skip);
        assert_eq!(enter, ResolveAction::Enter);
    }
}

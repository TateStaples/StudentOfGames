// subgame_solving.rs - Subgame construction and k-cover algorithm
//
// This module implements Knowledge-Limited Unfrozen Subgame Solving (KLUSS)
// and the k-cover algorithm for reasoning about knowledge without enumerating
// common-knowledge sets.

use crate::obscuro_core::*;
use std::collections::{HashMap, HashSet};

// ============================================================================
// K-Cover Algorithm
// ============================================================================

/// Find all histories that are believed possible up to order k
///
/// This implements the key insight of avoiding common-knowledge enumeration:
/// - k=1: What I believe the state could be
/// - k=2: What I know the opponent believes the state could be  
/// - k=3: What the opponent thinks I believe...
///
/// By limiting k, we prune states that are "too far" in the knowledge hierarchy
/// and keep only relevant states.
pub fn k_cover<G: Game>(
    root_histories: Vec<History<G>>,
    target_trace: G::Trace,
    mut player: Player,
    k: usize,
) -> Vec<History<G>> {
    if root_histories.is_empty() || k == 0 {
        return vec![];
    }

    let mut current_histories = root_histories;
    let mut search_traces = HashSet::from([target_trace]);
    let mut found_histories = Vec::new();

    // Iterate k times, alternating between players
    for _ in 0..k {
        let mut next_histories = Vec::new();
        let mut next_search_traces = HashSet::new();

        for history in current_histories {
            let (returned_history, found, new_traces) =
                k_cover_recursive(history, &search_traces, player);

            if let Some(h) = returned_history {
                next_histories.push(h);
            }

            found_histories.extend(found);
            next_search_traces.extend(new_traces);
        }

        current_histories = next_histories;
        search_traces = next_search_traces;
        player = player.other();
    }

    found_histories
}

/// Recursive helper for k-cover
///
/// Returns:
/// - Option<History>: History to continue searching from (or None if pruned)
/// - Vec<History>: Histories that matched the search traces
/// - HashSet<Trace>: New traces to search for in next iteration
fn k_cover_recursive<G: Game>(
    mut history: History<G>,
    target_traces: &HashSet<G::Trace>,
    player: Player,
) -> (Option<History<G>>, Vec<History<G>>, HashSet<G::Trace>) {
    // Terminal nodes can't be explored further
    if matches!(history, History::Terminal { .. }) {
        return (Some(history), vec![], HashSet::new());
    }

    let my_trace = history.trace(player);

    // Check if this trace matches any target
    let matches: Vec<std::cmp::Ordering> = target_traces
        .iter()
        .filter_map(|t| my_trace.partial_cmp(t))
        .collect();

    // If we found a match
    if matches.contains(&std::cmp::Ordering::Equal) {
        // Return this history as found, and add opponent's view to search
        let opponent_trace = history.trace(player.other());
        return (None, vec![history], HashSet::from([opponent_trace]));
    }

    // If this could lead to a match (partial order indicates containment)
    if !matches.is_empty() && matches.iter().all(|o| *o != std::cmp::Ordering::Greater) {
        // Explore children if expanded
        if let History::Expanded { children, .. } = &mut history {
            // Take ownership of children temporarily
            let children_vec = std::mem::take(children);

            // Process each child
            let (new_children, all_found, all_traces): (Vec<_>, Vec<_>, HashSet<_>) =
                children_vec.into_iter().fold(
                    (Vec::new(), Vec::new(), HashSet::new()),
                    |(mut cs, mut fs, mut ts), (action, child)| {
                        let (returned, found, traces) =
                            k_cover_recursive(child, target_traces, player);

                        if let Some(h) = returned {
                            cs.push((action, h));
                        }
                        fs.extend(found);
                        ts.extend(traces);

                        (cs, fs, ts)
                    },
                );

            // Restore children
            if let History::Expanded { children, .. } = &mut history {
                *children = new_children;
            }

            return (Some(history), all_found, all_traces);
        }
    }

    // No match and no exploration possible
    (Some(history), vec![], HashSet::new())
}

// ============================================================================
// Subgame Construction (KLUSS)
// ============================================================================

/// Construct a subgame using Knowledge-Limited Unfrozen Subgame Solving
///
/// This is the main entry point for building a subgame to solve.
pub fn construct_subgame<G: Game>(
    old_tree: Vec<History<G>>,
    observation: G::Trace,
    player: Player,
    min_positions: usize,
    k_depth: usize,
) -> HashMap<G::Trace, (Probability, Reward, Vec<History<G>>)> {
    // Step 1: Find relevant histories using k-cover
    let mut covered = k_cover(old_tree, observation.clone(), player, k_depth);

    // Step 2: Normalize probabilities
    let total_prob: Probability = covered.iter().map(|h| h.net_reach_prob()).sum();
    if total_prob > 0.0 {
        for h in &mut covered {
            let current_prob = h.net_reach_prob();
            h.set_reach_prob(player, current_prob / total_prob);
        }
    }

    // Step 3: Group by opponent's trace
    let mut positions: HashMap<G::Trace, (Probability, Reward, Vec<History<G>>)> =
        HashMap::new();

    for history in covered {
        let opponent = player.other();
        let opp_trace = history.trace(opponent);
        let prob = history.net_reach_prob();

        let value = match &history {
            History::Terminal { payoff, .. } => *payoff,
            History::Visited { payoff, .. } => *payoff,
            History::Expanded { game, .. } => game.evaluate(),
        };

        positions
            .entry(opp_trace)
            .and_modify(|(p, v, histories)| {
                *p += prob;
                *v = (*v * histories.len() as f64 + value) / (histories.len() as f64 + 1.0);
                histories.push(history.clone());
            })
            .or_insert((prob, value, vec![history]));
    }

    // Step 4: Sample additional positions if needed
    let current_count = positions.values().map(|(_, _, v)| v.len()).sum::<usize>();

    if current_count < min_positions {
        sample_additional_positions(&mut positions, observation, player, min_positions);
    }

    positions
}

/// Sample additional positions to ensure minimum coverage
fn sample_additional_positions<G: Game>(
    positions: &mut HashMap<G::Trace, (Probability, Reward, Vec<History<G>>)>,
    observation: G::Trace,
    player: Player,
    min_positions: usize,
) {
    let current_count = positions.values().map(|(_, _, v)| v.len()).sum::<usize>();
    let needed = min_positions.saturating_sub(current_count);

    if needed == 0 {
        return;
    }

    // Get existing game identifiers to avoid duplicates
    let existing_ids: HashSet<u64> = positions
        .values()
        .flat_map(|(_, _, histories)| histories.iter().map(|h| h.identifier()))
        .collect();

    let mut sampler = G::sample_positions(observation);
    let mut added = 0;

    while added < needed {
        if let Some(game) = sampler.next() {
            let id = game.identifier();

            // Skip if we already have this position
            if existing_ids.contains(&id) {
                continue;
            }

            let history = History::new(game.clone());
            let opp_trace = game.trace(player.other());
            let value = game.evaluate();

            positions
                .entry(opp_trace)
                .and_modify(|(p, v, histories)| {
                    // Uniform probability for sampled positions
                    *p += 1.0 / min_positions as f64;
                    *v = (*v * histories.len() as f64 + value) / (histories.len() as f64 + 1.0);
                    histories.push(history.clone());
                })
                .or_insert((1.0 / min_positions as f64, value, vec![history]));

            added += 1;
        } else {
            // No more positions available
            break;
        }
    }

    // Renormalize probabilities
    let total_prob: Probability = positions.values().map(|(p, _, _)| *p).sum();
    if total_prob > 0.0 {
        for (prob, _, _) in positions.values_mut() {
            *prob /= total_prob;
        }
    }
}

// ============================================================================
// Utilities
// ============================================================================

/// Filter old tree to keep only relevant parts for new subgame
pub fn prune_old_tree<G: Game>(
    old_tree: Vec<History<G>>,
    observation: G::Trace,
    player: Player,
) -> Vec<History<G>> {
    // Simple pruning: keep histories that match the observation
    old_tree
        .into_iter()
        .filter(|h| {
            let trace = h.trace(player);
            trace.partial_cmp(&observation).is_some()
        })
        .collect()
}

/// Compute distance in knowledge hierarchy between two traces
///
/// Returns the minimum k such that the traces are distinguishable with k-order knowledge
pub fn knowledge_distance<T: PartialOrd>(trace1: &T, trace2: &T) -> Option<usize> {
    match trace1.partial_cmp(trace2) {
        Some(std::cmp::Ordering::Equal) => Some(0),
        Some(_) => Some(1),
        None => None, // Not comparable
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k_cover_empty() {
        // Mock implementation would go here
        // For now, just verify function signature works
        let histories: Vec<History<DummyGame>> = vec![];
        let trace = ();
        let result = k_cover(histories, trace, Player::P1, 3);
        assert_eq!(result.len(), 0);
    }

    // Dummy game type for testing
    #[derive(Clone)]
    struct DummyGame;

    impl Game for DummyGame {
        type State = ();
        type Action = ();
        type Observation = ();
        type Trace = ();

        fn new() -> Self {
            DummyGame
        }
        fn active_player(&self) -> Player {
            Player::P1
        }
        fn is_terminal(&self) -> bool {
            false
        }
        fn payoff(&self) -> Reward {
            0.0
        }
        fn legal_actions(&self) -> Vec<Self::Action> {
            vec![()]
        }
        fn apply_action(&self, _: &Self::Action) -> Self {
            DummyGame
        }
        fn get_observation(&self, _: Player) -> Self::Observation {}
        fn trace(&self, _: Player) -> Self::Trace {}
        fn identifier(&self) -> u64 {
            0
        }
        fn evaluate(&self) -> Reward {
            0.0
        }
        fn sample_positions(_: Self::Trace) -> Box<dyn Iterator<Item = Self>> {
            Box::new(std::iter::empty())
        }
        fn encode(&self) -> Self::State {}
        fn decode(_: &Self::State) -> Self {
            DummyGame
        }
    }
}

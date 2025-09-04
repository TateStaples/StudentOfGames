use std::hash::Hash;

pub type ActionId = usize;
pub type StateId = usize;  // TODO: idk how to index in generic format
pub type Reward = f32;
pub type Probability = f32;
pub type Belief<G: ImperfectGame, Range> = (G, [Range; 2]);

pub trait HasTurnOrder: Eq + Clone + Copy + std::fmt::Debug {
    fn prev(&self) -> Self;
    fn next(&self) -> Self;
}

pub type PrivateObservation = usize;
pub type PublicObservation = usize;

pub trait AbstractPolicy: Clone {
    fn new() -> Self;
    fn eval(&self, action_id: crate::types::ActionId) -> crate::types::Probability;
    fn sample(&self) -> crate::types::ActionId;
    fn uniform() -> Self;
    fn mix_in(&self, other: &Self, p: crate::types::Probability) -> Self;
}
pub trait AbstractRange: Clone {
    fn new() -> Self;
    fn eval(&self, state_id: crate::types::StateId) -> crate::types::Probability;
    fn mix_in(&self, other: &Self, p: crate::types::Probability) -> Self;
}

pub trait AbstractCounterfactual: Clone {
    fn new() -> Self;
    fn outcome(value: crate::types::Reward) -> Self;
    fn eval(&self, state_id: crate::types::StateId) -> crate::types::Reward;
}

pub trait Game: Eq + Hash + Clone + std::fmt::Debug + Send {
    type PlayerId: HasTurnOrder;  // The playerId enum must have .next() and .prev()
    type Action: Eq + Clone + Copy + std::fmt::Debug + Into<usize> + From<usize>;
    type ActionIterator: Iterator<Item = Self::Action>;

    const MAX_TURNS: usize;
    const NUM_PLAYERS: usize;
    
    fn new() -> Self;  // Default initialization
    fn player(&self) -> Self::PlayerId;  // The active player
    fn is_over(&self) -> bool;
    fn reward(&self, player_id: Self::PlayerId) -> f32;  // The reward for the player getting to active state.
    fn iter_actions(&self) -> Self::ActionIterator;
    fn step(&mut self, action: &Self::Action) -> (PublicObservation, PrivateObservation);  // Implement action and return whether done
    fn print(&self);  // Output the game state
}

pub trait FixedGame<const A: usize, const S: usize>: Game {
    const MAX_NUM_ACTIONS: usize = A;
    const HIDDEN_STATES: usize = S;
}

pub trait ImperfectGame: Game {
    fn sample_state(public_information: Vec<PublicObservation>) -> Self;  // Sample a state from the public information
    fn transition(&self, public_observation: PublicObservation) -> &Self;
}

use once_cell::sync::Lazy;

use crate::game::{Game, ImperfectGame};
use crate::game_tree::{PrivateNode, PrivateNodeId};
use crate::policies::Prior;
use crate::types::{AbstractCounterfactual, AbstractPolicy, AbstractRange, Belief};

// CFR+ (populate SearchStatistics): belief down and counterfactual values (for given policy) up
// DeepStack: opp_range, reach prob, range, avg reach, values, regrets, avg_regrets [repeat]
pub(crate) fn cfr<'a, G: ImperfectGame + 'a, N: ImperfectNode<'a, G, Counterfactuals, Range, Policy>, P: Prior<G, Counterfactuals, Range, Policy>, Counterfactuals: AbstractCounterfactual, Range: AbstractRange, Policy: AbstractPolicy>
(tree: &mut GameTree<'a, G, N>, node: &mut N, ranges: [Range; 2], prior: &P) -> Counterfactuals {
    // DeepStack order: opp_range √, strategy (reach probabilities), ranges, update avg_strat, terminal values, values, regrets, avg_values
    let evaluation = Lazy::new(|| {
        let belief: Belief<G, Range> = (node.public_state(), ranges.clone());
        let val = prior.eval(belief);
        val
    });
    // node.reset();

    // Note: DeepStack stores the average CFVs for later storage
    for (result, new_ranges) in node.iter_results(&ranges) { // FIXME: this doesn't allow for distinguishing terminal states
        // propagate search_stats back up
        match result {
            Some(NodeTransition::Edge(next)) => {
                let mut_next = tree.mut_node(next);
                let counterfactuals = cfr(tree, mut_next, new_ranges, prior);  // propagate down
                node.update_children(result, counterfactuals);  // TODO: update player details
            },
            Some(NodeTransition::Terminal(v)) => {
                node.update_children(result, Counterfactuals::outcome(*v));
            },
            None => {
                let (value, _) = evaluation.unwrap();
                node.update_children(None, value);
            }
            _ => {assert!(false, "Phantom data in transition map")}
        };
    }
    node.update_value();  // combine all values for calculation
    return node.value();
}

// Imperfect Information Extensive-Form
pub trait SearchStatistics<'a, N: PrivateNode<'a, G>, G: Game + 'a, Counterfactuals: crate::types::AbstractCounterfactual, Range: crate::types::AbstractRange, Policy: crate::types::AbstractPolicy> : PrivateNode<'a, G> {  // Stores the statistics for the search in CFR
    // DeepStack: range, values, regrets, reach prob [repeat]
    fn visits(&self) -> f32;                            // Number of visits for growth step
    fn add_visit(&mut self);                            // Add a new visit
    fn reward(&self) -> crate::types::Reward;                         // Reward for this action
    fn calc_policy(&self) -> Policy;                    // Calculated policy from CFR alg stats: r(s,a) = v(s,a) - EV(policy), Q(s,a) += r(s,a) [min value of 0], π(s,a) = percentage of Q
    fn reset(&mut self);                                // clear all statistics before a new search

    fn solved(&self) -> bool { false }                  // Reward is fixed (set in perfect info)
    fn player(&self) -> G::PlayerId;

    fn update_children(&mut self, child: Option<PrivateNodeId>, counterfactuals: Counterfactuals);
    fn update_value(&mut self);
}
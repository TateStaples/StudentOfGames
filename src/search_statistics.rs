use std::collections::{HashMap, HashSet};
use crate::game::Game;
use crate::game_tree::{ActionId, Counterfactuals, Node, NodeTransition, Outcome, StateId, TransitionMatrix};
use crate::policies::Policy;

pub type Probability = f32;
pub type Range = Vec<(StateId, Probability)>;
pub type Belief<G: Game, const A: usize, const S: usize> = (G::PublicInformation, [Range; 2]);
pub type ActionPolicy = Vec<(Probability, ActionId)>;
// consider making a torch implementation of this
pub trait SearchStatistics<'a, G: Game> {  // Stores the statistics for the search in CFR
    // DeepStack: range, values, regrets, reach prob
    fn initialize(&mut self, ranges: [Range; 2]);
    fn reset(&mut self);
    fn action_probability(&self, state_id: StateId, action_id: ActionId) -> Probability { self.cfr_policy(state_id)[action_id]}
    fn visits(&self) -> f32;
    fn solved(&self) -> bool { false }
    fn value(&self) -> Option<Counterfactuals>;
    fn cfr_policy(&self, state_id: StateId) -> ActionPolicy;
    fn range(&self, player: G) -> Range;

    fn update_nodes_below(&mut self, state_id: StateId, action_id: ActionId);
    fn update_action_quality(&mut self, state_id: StateId, action_id: ActionId, value: Outcome, p: Probability);
    fn normalize(&mut self);
    fn iter_results(&self, ranges: &[Range; 2]) -> impl Iterator<Item=(NodeTransition<G, Self>, [Range; 2], Vec<(StateId, ActionId, Probability)>)>;  // TODO
}
pub struct FixedStatistics<'a, G: Game, const A: usize, const S: usize> {
    node: TransitionMatrix<'a, G, A, S>,
    nodes_below: [[f32; A]; S],
    action_quality: [[f32; A]; S],
    visits: f32,
    aggregate_value: [Outcome; S],
}


impl<'a, G: Game, const A: usize, const S: usize> Node<'a, G> for FixedStatistics<'a, G, A, S> {
    fn new(public_information: G::PublicInformation, transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<G, Self>>) -> Self {
        Self {
            node: TransitionMatrix::new(public_information, transition_map),
            nodes_below: [[0.0; A]; S],
            action_quality: [[0.0; A]; S],
            visits: 0.0,
            aggregate_value: [0.0; S],
        }
    }

    fn leaf(&self) -> bool {
        self.node.leaf()
    }

    fn public_state(&self) -> &G::PublicInformation {
        self.node.public_state()
    }

    fn transition(&self, state_1: StateId, state_2: StateId, action_id: ActionId) -> (StateId, &NodeTransition<G, Self<>>) {
        self.node.transition(state_1, state_2, action_id).into()
    }

    fn children(&self) -> HashSet<NodeTransition<G, Self>> {
        self.node.children().into()
    }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G) -> Option<Self> {
        self.node.visit(active_state, other_state, action_id, new_state).into()
    }
}

impl<'a, N: Node<'a, G>, G: Game, const A: usize , const S: usize> SearchStatistics<'a, G> for FixedStatistics<'a, G, A, S> {
    fn initialize(&mut self, prior: &impl Policy<G, A, S>) {

    }

    fn reset(&mut self) {
        self.nodes_below = [[0.0; A]; S];
        self.action_quality = [[0.0; A]; S];  // TODO: populate quality with average value
        self.aggregate_value = self.average_value();
        self.visits = 1.0;
    }

    fn action_probability(&self, state_id: StateId, action_id: ActionId) -> Probability {
        let action_q = self.action_quality[state_id];
        action_q[action_id] / action_q.iter().sum()
    }

    fn visits(&self) -> f32 {
        self.nodes_below.iter().map(|x| x.iter().sum()).sum()
    }

    fn value(&self) -> Counterfactuals {  //
        self.aggregate_value.iter().map(|x| x/self.visits).collect()
    }

    fn cfr_policy(&self, state_id: StateId) -> ActionPolicy {
        let action_q = self.action_quality[state_id];
        let sum = action_q.iter().sum();
        action_q.iter().map(|x| x/sum).collect()
    }

    fn range(&self, player: G) -> Range {
        todo!()
    }

    fn update_nodes_below(&mut self, state_id: StateId, action_id: ActionId) {
        self.nodes_below[state_id][action_id] += 1.0;
    }

    fn update_action_quality(&mut self, state_id: StateId, action_id: ActionId, value: Outcome, p: Probability) {
        // r(s,a) = v(s,a) - EV(policy)
        // Q(s,a) += r(s,a) [min value of 0]
        let state = &mut self.action_quality[state_id];
        for a in 0..A {
            if a == action_id {
                state[a] += value;  // update my quality
            } else {
                state[a] -= value * p;  // decrement other quality
            }
        }
    }

    fn normalize(&mut self) {
        todo!()
    }

    fn iter_results(&self, ranges: [Range; 2]) -> &dyn Iterator<Item=(NodeTransition<G, Self>, [Range; 2], Vec<(StateId, StateId, ActionId, Probability)>)> {
        todo!()
    }
}

pub trait ImperfectNode<'a, G: Game>: Node<'a, G> + SearchStatistics<'a, G>{}
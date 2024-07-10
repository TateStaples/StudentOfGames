// Expand game tree with imperfect information and statistics for search

use crate::game::{Game, ImperfectGame};
use crate::game_tree::{PrivateNode, NodeTransition};
use crate::types::{AbstractCounterfactual, AbstractPolicy, AbstractRange, PrivateObservation, Reward, StateId};

// Imperfect Information Extensive-Form
pub trait SearchStatistics<'a, N: PrivateNode<'a, G>, G: Game + 'a, Counterfactuals: AbstractCounterfactual, Range: AbstractRange, Policy: AbstractPolicy> : PrivateNode<'a, G> {  // Stores the statistics for the search in CFR
    // DeepStack: range, values, regrets, reach prob [repeat]
    fn visits(&self) -> f32;                            // Number of visits for growth step
    fn add_visit(&mut self);                            // Add a new visit
    fn reward(&self) -> Reward;                         // Reward for this action
    fn calc_policy(&self) -> Policy;                    // Calculated policy from CFR alg stats: r(s,a) = v(s,a) - EV(policy), Q(s,a) += r(s,a) [min value of 0], π(s,a) = percentage of Q
    fn reset(&mut self);                                // clear all statistics before a new search

    fn solved(&self) -> bool { false }                  // Reward is fixed (set in perfect info)
    fn player(&self) -> G::PlayerId;

    fn update_children(&mut self, child: Option<&NodeTransition<'a, G, Self>>, counterfactuals: Counterfactuals);
    fn update_value(&mut self);
}

// Combine the transition behavior of a node with the statistics of the search
// Normal game tree with a blanket thrown over it
pub trait ImperfectTree<'a, G: ImperfectGame + 'a, N: PrivateNode<'a, G>, Range: AbstractRange, ActionPolicy: AbstractPolicy>{
    fn iter_results(&self, ranges: &[Range; 2]) -> impl Iterator<Item=(Option<&Self>, [Range; 2])>;
    fn histories(&self) -> Vec<N>;
    fn expand(&mut self, ranges: [Range; 2], policy: ActionPolicy) -> Option<&Self>;
}
//
/*
// Specific implementation for fixed size (state size and action size) games
#[derive(Clone, Copy)]
struct FixedRange<const S: usize> {
    range: [Probability; S]
} 
impl<const S: usize> AbstractRange for FixedRange<S> {
    fn new() -> Self { Self { range: [(1 / S) as Probability; S] }}
    fn eval(&self, state_id: StateId) -> Probability {
        self.range[state_id]
    }
    fn mix_in(&self, other: &Self, p: Probability) -> Self {
        todo!()
    }
}
#[derive(Clone, Copy)]
struct FixedCounterfactuals<const A: usize, const S: usize> {
    counterfactuals: [[Outcome; A]; S]
} 
impl<const A: usize, const S: usize> AbstractCounterfactual for FixedCounterfactuals<A, S> {
    fn new() -> Self { Self { counterfactuals: [[0.0; A]; S] }}
    fn eval(&self, state_id: StateId) -> Outcome {
        self.counterfactuals[state_id].iter().sum()
    }
}
#[derive(Clone, Copy)]
struct FixedActionPolicy<const A: usize> {
    policy: [Probability; A]
} 
impl<const A: usize> AbstractPolicy for FixedActionPolicy<A> {
    fn new() -> Self { Self { policy: [1.0 / A as Probability; A] }}
    fn eval(&self, action_id: ActionId) -> Probability {
        self.policy[action_id]
    }
    fn sample(&self) -> ActionId {
        todo!()
    }
}

pub struct FixedStatistics<'a, G: Game, const A: usize, const S: usize> {
    node: [[[Option<NodeTransition<'a, G, Self>>; A]; S]; S],
    nodes_below: [[f32; A]; S],
    action_quality: [[f32; A]; S],
    visits: f32,
    aggregate_value: [Outcome; S],
    ranges: [FixedRange<S>; 2]
}

impl<'a, G: Game, const A: usize, const S: usize> FixedStatistics<'a, G, A, S> {
    fn average_value(&self) -> [Outcome; S] {
        todo!()
    }
}
impl<'a, G: Game, const A: usize, const S: usize> 
    Node<'a, G> for FixedStatistics<'a, G, A, S> {
    fn leaf(&self) -> bool {
        self.node.leaf()
    }

    fn transition(&self, state_1: StateId, state_2: StateId, action_id: ActionId) -> (StateId, &NodeTransition<'a, G, Self>) {
        // self.node.transition(state_1, state_2, action_id).into()
        todo!()
    }

    fn children(&self) -> HashSet<&NodeTransition<'a, G, FixedStatistics<'a, G, A, S>>> {
        // self.node.children().into()
        todo!()
    }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G) -> Option<Self> {
        // self.node.visit(active_state, other_state, action_id, new_state).into()
        todo!()
    }
}

impl<'a, G: Game + 'a, const A: usize , const S: usize> 
    SearchStatistics<'a, G, FixedCounterfactuals<A, S>, FixedRange<S>, FixedActionPolicy<A>> for FixedStatistics<'a, G, A, S> {
    fn visits(&self) -> f32 {
        // self.nodes_below.iter().map(|x| x.iter().sum()).sum()
        todo!()
    }

    fn value(&self) -> Option<FixedCounterfactuals<A, S>> {  //
        // self.aggregate_value.iter().map(|x| x/self.visits).collect()
        todo!()
    }

    fn cfr_policy(&self, state_id: StateId) -> FixedActionPolicy<A> {
        let action_q = self.action_quality[state_id];
        // let sum = action_q.iter().sum();
        // action_q.iter().map(|x| x/sum).collect()
        todo!()
    }

    fn range(&self, player: G::PlayerId) -> FixedRange<S> {
        // self.ranges[player.into()].clone()
        todo!()
    }

    fn reset(&mut self) {
        self.nodes_below = [[0.0; A]; S];
        self.action_quality = [[0.0; A]; S];  // TODO: populate quality with average value
        self.aggregate_value = self.average_value();
        self.visits = 1.0;
    }

    fn update_children(&mut self, child: Option<&NodeTransition<'a, G, Self>>, counterfactuals: FixedCounterfactuals<A, S>) {
        todo!()
    }

    fn update_value(&mut self) {
        todo!()
    }
}
impl<'a, G: ImperfectGame + 'a, const A: usize, const S: usize> 
    ImperfectNode<'a, G, FixedCounterfactuals<A, S>, FixedRange<S>, FixedActionPolicy<A>> for FixedStatistics<'a, G, A, S> {
    fn public_state(&self) -> G::PublicInformation {
        todo!()
    }

    fn iter_results(&self, ranges: &[FixedRange<S>; 2]) -> impl Iterator<Item=(Option<&NodeTransition<'a, G, Self>>, [FixedRange<S>; 2])> {
        vec![].into_iter()  // TODO
    }
    
}
*/
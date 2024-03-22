use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::mem::size_of;
use crate::game_tree::NodeTransition::Undefined;
use crate::helpers::prelude::Game;

pub type ActionId = usize;
pub type StateId = usize;  // TODO: idk how to index in generic format
pub type Outcome = f32;
pub type Counterfactuals = Vec<Outcome>;  // is 2p0s this can be stored as a single array
pub type PublicInformation<G: Game> = G::PublicInformation;  // public and private information knowledge of player

pub(crate) struct GameTree<'a, G: Game, N: Node<'a, G>> {
    nodes: Vec<N>,
    root: &'a N,
}

impl <'a, G: Game, N: Node<'a, G>> GameTree<'a, G, N> {
    pub(crate) fn with_capacity(capacity: usize, root: N) -> Self {
        let mut new = Self {
            nodes: Vec::with_capacity(capacity),
            root: &root,
        };
        new.nodes.push(root);
        new
    }
    pub(crate) fn push(&mut self, node: N) { self.nodes.push(node); }
    pub(crate) fn mut_node(&mut self, node: &N) -> &mut N {
        unsafe {
            let node_ptr = node as *const isize;
            let vec_ptr = &self.nodes as *const isize;
            let diff = (node_ptr as isize - vec_ptr as isize) / size_of::<N>() as isize;
            if let Ok(usize_index) = diff.try_into() {
                if usize_index < self.nodes.len() {
                    self.nodes.get_mut(usize_index)
                } else {
                    panic!("Attempted to get a Node not in the tree");
                }
            }
            panic!("Attempted to get a Node not in the tree");
        }
    }
    pub(crate) fn root(&self) -> &N { self.node(self.root) }
    pub(crate) fn reroot(&mut self, new_root: &N) { self.root = new_root } // maybe we can clear space -> array of options
}

pub enum NodeTransition<'a, G: Game, N: Node<'a, G>> {
    Edge(&'a N),
    Terminal(Outcome),
    Undefined
    // Chance(Vec<Probability, NodeId>)
}

pub trait Node<'a, G: Game> {
    fn new(public_information: G::PublicInformation, transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<'a, G, Self>>) -> Self;
    fn empty(public_information: G::PublicInformation) -> Box<Self> { Self::new(public_information, HashMap::new()) }
    fn leaf(&self) -> bool;
    fn player(&self) -> G::PlayerId { self.public_state().player() }
    fn public_state(&self) -> &G::PublicInformation;
    fn transition(&self, state_1: StateId, state_2: StateId, action_id: ActionId) -> (StateId, &NodeTransition<'a, G, Self>);
    fn children(&self) -> HashSet<NodeTransition<'a, G, Self>>;
    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G) -> Option<Self>;
}

pub struct TransitionMatrix<'a, G: Game, const A: usize, const S: usize> {  // Decision and chance nodes
    public_state: PublicInformation<G>,      // public information state // player to move (can be chance player)
    transition_map: [[[NodeTransition<'a, G, Self>; A]; S]; S],   // payoff matrix
}

struct TransitionMap<'a, G: Game> {
    public_state: PublicInformation<G>,
    transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<'a, G, Self>>
}

impl<'a, G: Game, const A: usize, const S: usize> Node<'a, G> for TransitionMatrix<'a, G, A, S> {
    fn new(public_information: G::PublicInformation, transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<G, Self>>) -> Self {
        let mut map = [[[Undefined; A]; S]; S];
        for ((s1, s2, a), t) in transition_map {
            map[s1][s2][a] = t;
        }

        Self {
            public_state: public_information,
            transition_map: map,
        }

    }
    #[inline]
    fn leaf(&self) -> bool { self.transition_map.iter().flatten().any(|res| res == Undefined) }
    #[inline]
    fn public_state(&self) -> &G::PublicInformation { &self.public_state }
    #[inline]
    fn transition(&self, active_state: StateId, other_state: StateId, action_id: ActionId) -> (StateId, &NodeTransition<G, Self>) { (active_state, &self.transition_map[active_state][other_state][action_id]) }

    fn children(&self) -> HashSet<&NodeTransition<G, Self>> {
        self.transition_map.iter().flatten().map(|res| *res).collect()
    }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G) -> Option<Self> {
        let (new_id, current_result) = self.transition(active_state, other_state, action_id);
        let public = new_state.public_information();
        for transition in self.children() {
            if let NodeTransition::Edge(&next) = transition {
                self.transition_map[active_state][other_state][action_id] = NodeTransition::Edge(&next);
                return None;
            }
        }
        let new_node = Self::empty(new_state.public_information()).into();
        let result = NodeTransition::Edge(&new_node);
        self.transition_map[active_state][other_state][action_id] = result;
        Some(*new_node)
    }
}

impl<'a, G: Game> Node<'a, G> for TransitionMap<'a, G> {
    fn new(public_information: G::PublicInformation, transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<G, Self>>) -> Self {
        Self {
            public_state: public_information,
            transition_map
        }
    }

    fn leaf(&self) -> bool {
        self.children().iter().any(|x| x == Undefined)
    }

    fn public_state(&self) -> &G::PublicInformation { &self.public_state }

    fn transition(&self, state_1: StateId, state_2: StateId, action_id: ActionId) -> (StateId, &NodeTransition<'a, G, Self>) {
        return (state_1, self.transition_map.get(&(state_1, state_2, action_id)).unwrap_or(&Undefined))
    }

    fn children(&self) -> HashSet<NodeTransition<G, Self>> {
        todo!()
    }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G) -> Option<Self> {
        todo!()
    }
}

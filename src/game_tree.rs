use std::collections::HashSet;
use std::marker::PhantomData;
use crate::game::Game;
use crate::types::{Reward, PrivateObservation, PublicObservation, AbstractRange, ActionId};


#[derive(Clone, Copy)]
pub enum NodeTransition<'a, G: Game, N: PrivateNode<'a, G>> { // The outcome of action either leading to a new node or a terminal state
    Edge(&'a N),
    Terminal(Reward),
    Phantom(PhantomData<G>)
}
pub enum NodeType {
    Inner,
    Leaf,
    Terminal
}
pub trait PublicNode<'a, Node: PrivateNode<'a, G>, G: Game, Range: AbstractRange> {
    fn sample(&self) -> Node;
    fn location(&self) -> NodeType;
    fn transition(&self, public_observation: PublicObservation) -> &Option<NodeTransition<'a, G, N>>;
    fn range(&self, player: G::PlayerId) -> Range;
}
// Node Store: priv / pub update (for reconstruction), transition: priv -> self, reward
pub trait PrivateNode<'a, G: Game> : Sized {  // A node in a game tree
    // fn new(public_information: G::PublicInformation, transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<'a, G, Self>>) -> Self;
    fn empty() -> Self;  // todo: does this make sense? could be useful for prior eval
    fn reward(&self) -> Reward;
    fn location(&self) -> NodeType;
    fn player(&self) -> G::PlayerId {   // The acting player for the given state
        // self.public_state().player() 
        todo!()
    }
    fn actions(&self) -> Iterator<Item = ActionId>;  // The possible actions for the given state
    fn game(&self) -> G;  // The game state
    fn transition(&self, update: PrivateObservation) -> &Option<NodeTransition<'a, G, Self>>;  // get the result fo 
    fn children(&self) -> HashSet<&NodeTransition<'a, G, Self>>;
    fn visit(&mut self, private_observation: PrivateObservation, new_state: G, edge: &mut NodeTransition<'a, G, Self>);
}

/*
pub struct TransitionMatrix<'a, G: Game, const A: usize, const S: usize> {  // Decision and chance nodes
    transition_map: [[[Option<NodeTransition<'a, G, Self>>; A]; S]; S],   // payoff matrix
}

impl<'a, G: Game, const A: usize, const S: usize> 
    Node<'a, G> for TransitionMatrix<'a, G, A, S> {

    #[inline]
    fn leaf(&self) -> bool { 
        // self.transition_map.iter().flatten().any(|res| res == Undefined) 
        todo!()
    }

    #[inline]
    fn transition(&self, active_state: StateId, other_state: StateId, action_id: ActionId) -> (StateId, &Option<NodeTransition<'a, G, Self>>) {
        // (active_state, &self.transition_map[active_state][other_state][action_id])
        todo!()
    }

    fn children(&self) -> HashSet<&NodeTransition<'a, G, Self>> {
        // self.transition_map.iter().flatten().map(|res| *res).collect()
        todo!()
    }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G) -> Option<Self> {
        // let (new_id, current_result) = self.transition(active_state, other_state, action_id);
        // let public = new_state.public_information();
        // for transition in self.children() {
        //     if let NodeTransition::Edge(&next) = transition {
        //         self.transition_map[active_state][other_state][action_id] = NodeTransition::Edge(&next);
        //         return None;
        //     }
        // }
        // let new_node = *Self::empty(new_state.public_information());
        // let result = NodeTransition::Edge(&new_node);
        // self.transition_map[active_state][other_state][action_id] = result;
        // Some(new_node)
        todo!()
    }
}


// struct TransitionMap<'a, G: Game> {
//     transition_map: HashMap<ActionId, NodeTransition<'a, G, Self>>
// }

impl<'a, G: Game> Node<'a, G> for 
    TransitionMap<'a, G> {

    fn leaf(&self) -> bool {
        // self.children().iter().any(|x| x == Undefined)
        todo!()
    }
    
    fn transition(&self, state_1: StateId, state_2: StateId, action_id: ActionId) -> (StateId, &NodeTransition<'a, G, Self>) {
        return (state_1, self.transition_map.get(&(state_1, state_2, action_id)).unwrap_or(&Undefined))
    }

    fn children(&self) -> HashSet<&NodeTransition<'a, G, Self>> {
        todo!()
    }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G) -> Option<Self> {
        todo!()
    }
}
*/
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::obscuro::utils::*;
use crate::obscuro::info::*;

// ---------- History ----------
// #[derive(PartialEq)]
pub enum History<G: Game> {
    Terminal { payoff: Reward },
    New { state: Box<G::State> },
    Visited { state: Box<G::State>, reach: HashMap<Player, Probability> },
    Expanded { info: InfoPtr<G::Action, G::Trace>, reach: HashMap<Player, Probability>, children: Vec<(G::Action, History<G>)>, player: Player },
}

impl <G: Game> Debug for History<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self { 
            History::Terminal { payoff } => write!(f, "Terminal({:?})", payoff),
            History::New { state } => write!(f, "New()"),
            History::Visited { state, reach } => write!(f, "Visited({:?})", reach),
            History::Expanded { info, reach, children, player } => write!(f, "Expanded({:?}, {:?}, {:?}, {:?})", info, reach, children, player),
        }
    }
}

impl<G: Game> Clone for History<G> {
    fn clone(&self) -> Self {
        match self {
            History::Terminal { payoff } => History::Terminal { payoff: *payoff },
            History::New { state } => History::New { state: state.clone() },
            History::Visited { state, reach } => History::Visited { state: state.clone(), reach: reach.clone() },
            History::Expanded { info, reach, children, player } => History::Expanded {
                info: info.clone(),
                reach: reach.clone(),
                children: children.clone(),
                player: *player,
            },
        }
    }
}

impl<G: Game> History<G> {
    pub fn new(state: G::State) -> Self { History::New { state: Box::new(state) } }

    pub fn payoff(&self) -> Reward {
        match self {
            History::Terminal { payoff } => *payoff,
            History::New { state } | History::Visited { state, .. } => G::decode(state).evaluate(),
            History::Expanded { info, .. } => info.borrow().policy.expectation(),
        }
    }

    pub fn player(&self) -> Player {
        match self {
            History::Terminal { .. } => panic!("terminal has no player"),
            History::New { state } | History::Visited { state, .. } => G::decode(state).active_player(),
            History::Expanded { info, .. } => info.borrow().player,
        }
    }

    pub fn trace(&self) -> G::Trace {
        match self {
            History::Terminal { .. } => unimplemented!(),
            History::New { state } | History::Visited { state, .. } => {
                let g = G::decode(state);
                g.trace(g.active_player())
            }
            History::Expanded { info, .. } => info.borrow().trace.clone(),
        }
    }

    pub fn expand(&mut self, infosets: &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>) {
        if let History::Visited { state, .. } = self {
            let game = G::decode(state);
            let player = game.active_player();
            let actions = game.available_actions();

            let mut kids: Vec<(G::Action, History<G>)> = Vec::with_capacity(actions.len());
            let mut succ_traces: Vec<Option<G::Trace>> = Vec::with_capacity(actions.len());
            let mut succ_ptrs: Vec<Option<InfoPtr<G::Action, G::Trace>>> = Vec::with_capacity(actions.len());
            let mut succ_alt: Vec<Option<Reward>> = Vec::with_capacity(actions.len());

            for a in actions.iter() {
                let next = game.play(a);
                let child_trace = next.trace(next.active_player());
                let alt = next.evaluate();
                succ_traces.push(Some(child_trace.clone()));
                succ_ptrs.push(infosets.get(&child_trace).cloned());
                succ_alt.push(Some(alt));
                kids.push((a.clone(), History::new(next.encode())));
            }

            // Create/get this infoset
            let this_trace = game.trace(player);
            let rc: InfoPtr<G::Action, G::Trace> = if let Some(rc) = infosets.get(&this_trace) {
                rc.clone()
            } else {
                let info = todo!();//Info::new(actions.clone(), this_trace.clone(), player);
                let rc = Rc::new(RefCell::new(info));
                infosets.insert(this_trace.clone(), rc.clone());
                rc
            };

            // save successors
            {
                let mut info = rc.borrow_mut();
                info.succ_traces = succ_traces;
                info.succ_ptrs = succ_ptrs;
                info.succ_alt = succ_alt;
            }

            *self = History::Expanded { info: rc, reach: HashMap::new(), children: kids, player };
        }
    }
    
    pub fn reach_prob(&self, player: Player) -> Probability {
        todo!()
    }
    
    pub fn compare(&self, trace: G::Trace) -> Option<Ordering> {
        None
    }
}
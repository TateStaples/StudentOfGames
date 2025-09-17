use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;
use log::info;
use crate::utils::*;
use crate::info::*;
use crate::policy::Policy;

// ---------- History ----------
// #[derive(PartialEq)]
pub enum History<G: Game> {
    Terminal { payoff: Reward},
    Visited { state: G::State, payoff: Reward, reach: HashMap<Player, Probability> },
    Expanded { info: InfoPtr<G::Action, G::Trace>, reach: HashMap<Player, Probability>, 
        children: Vec<(G::Action, History<G>)>, player: Player, villan_trace: G::Trace },
}

impl<G: Game> Hash for History<G> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let hash_item = self.identifier();
        hash_item.hash(state)
    }
}

impl <G: Game> Debug for History<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self { 
            History::Terminal { payoff, .. } => write!(f, "Terminal({:?})", payoff),
            History::Visited { state, .. } => write!(f, "Visited({:?})", G::decode(&state.clone()).trace(Player::P1)),
            History::Expanded { info, reach, player, children, .. } => {
                // trace, actions, distribution
                let info = &info.borrow();
                let trace = info.trace.clone();
                let policy = info.policy.clone();
                write!(f, "Expanded({:?}, {:?}, {})", trace, player, children.len())
                // write!(f, "Expanded({:?}, {:?}, {:?}, {:.1?})", trace, player, policy, reach)
            },
        }
    }
}


impl<G: Game> History<G> {
    pub fn new(game: G, reach: HashMap<Player, Probability>) -> Self {
        let payoff = game.evaluate();
        if game.is_over() {
            return History::Terminal { payoff };
        }
        let state = game.encode();
        History::Visited { state, payoff, reach }
    }

    pub fn print(&self) {
        println!("{:?}", self);
    }

    pub fn print_family(&self) {
        self.print_family_rec(0, 5);
    }
    
    pub fn size(&self) -> usize {
        match self {
            History::Terminal { .. } | History::Visited { .. } => 1,
            History::Expanded { children, .. } => 1 + children.iter().map(|(_, h)| h.size()).sum::<usize>(),
        }
    }
    fn print_family_rec(&self, tab_level: usize, depth: usize) {
        print!("{}", "    ".repeat(tab_level));
        self.print();
        if depth == 0 { return; }
        if let History::Expanded { children, .. } = self {
            for (a, h) in children.iter() {
                // print!("{:?} -> ", a);
                print!("{}", tab_level);
                h.print_family_rec(tab_level + 1, depth - 1);
            }
        }
    }

    pub fn payoff(&self) -> Reward {
        match self {
            History::Terminal { payoff, .. } | History::Visited { payoff, .. } => *payoff,
            History::Expanded { info, .. } => info.borrow_mut().policy.expectation(),
        }
    }

    pub fn player(&self) -> Player {
        match self {
            History::Terminal { .. } => panic!("terminal has no player"),
            History::Visited { state, .. } => G::decode(state).active_player(),
            History::Expanded { info, .. } => info.borrow().player,
        }
    }

    pub fn trace(&self) -> G::Trace {
        match self {
            History::Terminal { .. } => unimplemented!(),
            History::Visited { state, .. } => {
                let g = G::decode(state);
                g.trace(g.active_player())
            }
            History::Expanded { info, .. } => info.borrow().trace.clone(),
        }
    }

    pub fn expand(&mut self, infosets: &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>) {
        let me = self.player();
        if let History::Visited { state, reach, .. } = self {
            let game = G::decode(state);
            let player = game.active_player();
            let villan = player.other();
            let villan_trace = game.trace(villan);
            let actions = game.available_actions();

            let mut kids: Vec<(G::Action, History<G>)> = Vec::with_capacity(actions.len());

            for a in actions.iter() {
                let next = game.play(a);
                // let child_trace = next.trace(next.active_player());
                // let alt = next.evaluate();
                let mut next_reach = reach.clone();
                next_reach.entry(me).and_modify(|e| *e *= 1.0/actions.len() as Probability).or_insert(1.0/actions.len() as Probability);
                let child = History::new(next, next_reach);
                kids.push((a.clone(), child));
            }
            // Create/get this infoset
            let this_trace = game.trace(player);
            let rc: InfoPtr<G::Action, G::Trace> = if let Some(rc) = infosets.get(&this_trace) {
                rc.clone()
            } else {
                let info = Info::from_policy(
                    Policy::from_rewards(kids.iter().map(|(a, h)| {
                        (a.clone(), h.payoff())
                    }).collect(), player), this_trace.clone(), player);
                let rc = Rc::new(RefCell::new(info));
                infosets.insert(this_trace.clone(), rc.clone());
                rc
            };

            *self = History::Expanded { info: rc, reach: HashMap::new(), children: kids, player, villan_trace };
            // print!("Expanding: "); self.print_family();
        } else {
            panic!("Can only expand a visited state");
        }
    }
    
    pub fn full_expand(&mut self, infosets: &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>) {
        if let History::Visited { .. } = self {
            self.expand(infosets);
        }
        if let History::Expanded { children, .. } = self {
            for (_, h) in children.iter_mut() {
                h.full_expand(infosets);
            }
        }
    }
    
    pub fn reach_prob(&self, player: Player) -> Probability {
        match self {
            History::Terminal { .. } => unimplemented!("You should not be here"),
            History::Visited { reach, .. } | History::Expanded { reach, ..} => *reach.get(&player).unwrap_or(&1.0)
        }
    }
    pub fn net_reach_prob(&self) -> Probability {
        match self {
            History::Terminal { .. } => unimplemented!("You should not be here"),
            History::Visited { reach, .. } | History::Expanded { reach, ..} => reach.values().product(),
        }
    }
    
    pub fn players_view(&self, player: Player) -> G::Trace {
        let (hero_trace, villan_trace) = self.identifier();
        if player == self.player() {
            hero_trace
        } else {
            villan_trace
        }
    }
    
    pub fn identifier(&self) -> (G::Trace, G::Trace) {
        match self {
            History::Terminal { .. } => panic!("Why do you want my hash!"),
            History::Visited { state, .. } => {
                let g = G::decode(state);
                (g.trace(g.active_player()), g.trace(g.active_player().other()))
            },
            History::Expanded { info, villan_trace, .. } => {
                (info.borrow().trace.clone(), villan_trace.clone())
            },
        }
    }
}
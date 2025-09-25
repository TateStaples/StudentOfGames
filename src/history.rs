use crate::info::*;
use crate::policy::Policy;
use crate::utils::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

// ---------- History ----------
// #[derive(PartialEq)]
pub enum History<G: Game> {
    Terminal { payoff: Reward},
    Visited { state: G::State, payoff: Reward, reach: HashMap<Player, Probability> },
    Expanded { info: InfoPtr<G::Action, G::Trace>, reach: HashMap<Player, Probability>, 
        children: Vec<(G::Action, History<G>)>, player: Player, villan_trace: G::Trace },
}

impl<G: Game> History<G> {
    /// Constructor from the game state and reach probabilities
    pub fn new(game: G, reach: HashMap<Player, Probability>) -> Self {
        let payoff = game.evaluate();
        if game.is_over() {
            return History::Terminal { payoff };
        }
        let state = game.encode();
        History::Visited { state, payoff, reach }
    }
    /// Growing Tree: take a leaf node and grow its children into new nodes
    pub fn expand(&mut self, infosets: &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>) {
        let me = self.player();
        if let History::Visited { state, reach, .. } = self {
            let game = G::decode(state);
            let hero = game.active_player();
            // let villan = hero.other();
            let (hero_trace, villan_trace) = game.identifier();
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
            // Create/get this infosetx
            let rc: InfoPtr<G::Action, G::Trace> = if let Some(rc) = infosets.get(&hero_trace) {
                rc.clone()
            } else {
                let info = Info::from_policy(
                    Policy::from_rewards(kids.iter().map(|(a, h)| {
                        (a.clone(), h.payoff())
                    }).collect(), hero), hero_trace.clone(), hero);
                let rc = Rc::new(RefCell::new(info));
                infosets.insert(hero_trace.clone(), rc.clone());
                rc
            };

            *self = History::Expanded { info: rc, reach: HashMap::new(), children: kids, player: hero, villan_trace };
            // print!("Expanding: "); self.print_family();
        } else {
            panic!("Can only expand a visited state");
        }
    }
    /// After lowering the tree, rescale probabilities
    pub fn renormalize_reach(&mut self, total_prob: Probability) {
        match self {
            History::Terminal { .. } => unimplemented!("You should not be here"),
            History::Visited { reach, .. } | History::Expanded { reach, ..} => {
                for (_, p) in reach.iter_mut() {
                    *p /= total_prob;
                }
            }
        }
    }
    // ---------- Enum Agnostic Getters --------- //
    /// What the expectation (p1=+) is from the current position
    pub fn payoff(&self) -> Reward {
        match self {
            History::Terminal { payoff, .. } | History::Visited { payoff, .. } => *payoff,
            History::Expanded { info, .. } => info.borrow_mut().policy.expectation(),
        }
    }
    /// Who the given player is
    pub fn player(&self) -> Player {
        match self {
            History::Terminal { .. } => panic!("terminal has no player"),
            History::Visited { state, .. } => G::decode(state).active_player(),
            History::Expanded { info, .. } => info.borrow().player,
        }
    }
    /// What the given player knows
    pub fn players_view(&self, player: Player) -> G::Trace {
        let (hero_trace, villan_trace) = self.identifier();
        if player == self.player() {
            hero_trace
        } else {
            villan_trace
        }
    }
    /// What the acting player knows
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
    /// How likely is a given player to choose to be in the position
    pub fn reach_prob(&self, player: Player) -> Probability {
        match self {
            History::Terminal { .. } => unimplemented!("You should not be here"),
            History::Visited { reach, .. } | History::Expanded { reach, ..} => *reach.get(&player).unwrap_or(&1.0)
        }
    }
    /// How likely is the game to end in this position (product of all players chocies)
    pub fn net_reach_prob(&self) -> Probability {
        match self {
            History::Terminal { .. } => unimplemented!("You should not be here"),
            History::Visited { reach, .. } | History::Expanded { reach, ..} => reach.values().product(),
        }
    }
    // ---------- Utils --------- //
    /// Debug tool to grow the full game tree (don't use on non-trivial games)
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
    /// Fully identify a history by what both players know
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
    pub fn print(&self) {
        println!("{:?}", self);
    }
    /// Print the whole game tree
    pub fn print_family(&self) {
        self.print_family_rec(0, 5);
    }
    /// Recursive step in printing the whole game tree
    pub fn print_family_rec(&self, tab_level: usize, depth: usize) {
        print!("{}", "    ".repeat(tab_level));
        self.print();
        if depth == 0 { return; }
        if let History::Expanded { children, .. } = self {
            for (_, h) in children.iter() {
                print!("{}", tab_level);
                // print!("{:?} -> ", a);
                h.print_family_rec(tab_level + 1, depth - 1);
            }
        }
    }
    /// Recursively find the size of the whole game tree
    pub fn size(&self) -> usize {
        match self {
            History::Terminal { .. } | History::Visited { .. } => 1,
            History::Expanded { children, .. } => 1 + children.iter().map(|(_, h)| h.size()).sum::<usize>(),
        }
    }
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
            History::Expanded { info, player, children, .. } => {
                // trace, actions, distribution
                let info = &info.borrow();
                let trace = info.trace.clone();
                let child_info = children.iter().map(|(a, _)|a).collect::<Vec<_>>();
                // let child_info = children.len();
                // let policy = info.policy.clone();
                write!(f, "Expanded({:?}, {:?}, {:?})", trace, player, child_info)
                // write!(f, "Expanded({:?}, {:?}, {:?}, {:.1?})", trace, player, policy, reach)
            },
        }
    }
}

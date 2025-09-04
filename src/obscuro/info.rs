use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::rc::Rc;
use crate::obscuro::utils::*;
use crate::obscuro::policy::*;
use crate::obscuro::history::*;

// ---------- Info (an infoset) ----------
pub type InfoPtr<A, T> = Rc<RefCell<Info<A, T>>>;

/// Structure representing what is known given a set of indistinguishable histories (for acting player)
#[derive(Debug)]
pub struct Info<A: ActionI, T: TraceI> {
    pub policy: Policy<A>,
    pub trace: T,
    pub player: Player,
    pub reach: HashMap<Player, Probability>,
    pub visited: bool,

    // Successor metadata for gift() and filter() - TODO: we might want to remove this
    pub succ_traces: Vec<Option<T>>,
    pub succ_ptrs: Vec<Option<InfoPtr<A, T>>>,
    pub succ_alt: Vec<Option<Reward>>,

    pub(crate) gift_cached: RefCell<Option<Reward>>,
}

impl<A: ActionI, T: TraceI> Info<A, T> {
    // pub fn new(actions: Vec<A>, trace: T, player: Player) -> Self {
    //     let n = actions.len();
    //     Info {
    //         policy: Policy::from_actions(actions, match player { Player::P1 => 1, Player::P2 => -1,
    //             _ => 1
    //         }),
    //         trace,
    //         player,
    //         reach: HashMap::new(),
    //         visited: false,
    //         succ_traces: vec![None; n],
    //         succ_ptrs: vec![None; n],
    //         succ_alt: vec![None; n],
    //         gift_cached: RefCell::new(None),
    //     }
    // }
    
    pub fn from_policy(policy: Policy<A>, trace: T, player: Player) -> Self {
        let n = policy.actions.len();
        Info {
            policy,
            trace,
            player,
            reach: HashMap::new(),
            visited: false,
            succ_traces: vec![None; n],
            succ_ptrs: vec![None; n],
            succ_alt: vec![None; n],
            gift_cached: RefCell::new(None),
        }
    }

    pub fn add_counterfactuals(&mut self, a: A, cfv: Reward) {
        let i = self.policy.actions.iter().position(|x| *x == a).unwrap();
        self.policy.expectations[i] = cfv;
    }

    pub fn update(&mut self) { self.policy.update(); }

    pub fn gift(&self) -> Reward {
        if let Some(v) = *self.gift_cached.borrow() { return v; }
        let mut seen = HashSet::<T>::new();
        let v = self.gift_inner(&mut seen);
        *self.gift_cached.borrow_mut() = Some(v);
        v
    }

    fn gift_inner(&self, seen: &mut HashSet<T>) -> Reward {
        if !seen.insert(self.trace.clone()) { return 0.0; } // guard

        let here = self.policy.expectation();
        let mut total = 0.0;

        for i in 0..self.policy.actions.len() {
            // local upside vs parent
            let child_val = if let Some(Some(child)) = self.succ_ptrs.get(i) {
                // recurse
                let v = child.borrow().policy.expectation();
                total += child.borrow().gift_inner(seen);
                v
            } else {
                self.succ_alt.get(i).and_then(|x| *x).unwrap_or(here)
            };

            if child_val > here {
                total += child_val - here;
            }
        }
        total
    }

    pub fn add_history<G: Game<Action=A, Trace=T>>(&mut self, h: &mut History<G>) {
        self.visited = true;
        match h {
            History::Expanded { reach, .. } | History::Visited { reach, .. } => {
                for (p,pr) in reach.iter() {
                    *self.reach.entry(*p).or_insert(0.0) += *pr;
                }
            }
            _ => {}
        }
    }
}
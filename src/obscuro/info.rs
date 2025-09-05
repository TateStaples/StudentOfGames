use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
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
    
    pub(crate) gift_cached: Option<Reward>,
}

impl<A: ActionI, T: TraceI> Info<A, T> {
    pub fn from_policy(policy: Policy<A>, trace: T, player: Player) -> Self {
        Info {
            policy,
            trace,
            player,
            reach: HashMap::new(),
            visited: false,
            gift_cached: None,
        }
    }

    // pub fn add_counterfactuals(&mut self, a: A, cfv: Reward) {
    //     self.policy.set_expectation(&a, cfv);
    // }

    /// Something to do with quantifying the uncertainty
    pub fn gift(&mut self) -> Reward {
        if let Some(v) = self.gift_cached{ return v; }
        let mut seen = HashSet::<T>::new();
        let v = self.gift_inner(&mut seen);
        self.gift_cached = Some(v);
        v
    }

    fn gift_inner(&self, seen: &mut HashSet<T>) -> Reward {
        0.0
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
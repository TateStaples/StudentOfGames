use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use crate::utils::*;
use crate::policy::*;
use crate::history::*;

// ---------- Info (an infoset) ----------
/// Datatype to allow all relevent history to see thier trace, policy, and player
pub type InfoPtr<A, T> = Rc<RefCell<Info<A, T>>>;  // All the things need to be able to reference their pol

/// Structure representing what is known given a set of indistinguishable histories (for acting player)
#[derive(Debug)]
pub struct Info<A: ActionI, T: TraceI> {
    pub policy: Policy<A>,
    pub trace: T,
    pub player: Player,
    pub reach: HashMap<Player, Probability>,
    pub gift_cached: Option<Reward>,
}

impl<A: ActionI, T: TraceI> Info<A, T> {
    /// Initialize an info to return all of these
    pub fn from_policy(policy: Policy<A>, trace: T, player: Player) -> Self {
        Info {
            policy,
            trace,
            player,
            reach: HashMap::new(),
            gift_cached: None,
        }
    }

    /// Something to do with quantifying the uncertainty
    pub fn gift(&mut self) -> Reward {  // TODO: make this work -> believe makes for better subgame solving
        if let Some(v) = self.gift_cached{ return v; }
        let mut seen = HashSet::<T>::new();
        let v = self.gift_inner(&mut seen);
        self.gift_cached = Some(v);
        v
    }

    fn gift_inner(&self, _: &mut HashSet<T>) -> Reward {
        0.0
    }

    /// Add another history into our set
    pub fn add_history<G: Game<Action=A, Trace=T>>(&mut self, h: &mut History<G>) {
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
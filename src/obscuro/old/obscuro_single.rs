#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::IteratorRandom;
use rand::thread_rng;

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

// ---------- Tunables ----------
const SOLVE_TIME_SECS: u64 = 2;
const MIN_INFO_SIZE: usize = 64;
const MAX_SUPPORT: usize = 3;

// ---------- Basic types ----------
pub type Reward = f64;
pub type Probability = f64;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Player { P1, P2 }

impl Player {
    #[inline] pub fn other(self) -> Player {
        match self { Player::P1 => Player::P2, Player::P2 => Player::P1 }
    }
}

// ---------- Traits the game must provide ----------
pub trait ActionI: Clone + Eq + Hash {}
impl<T: Clone + Eq + Hash> ActionI for T {}

pub trait TraceI: Clone + Eq + Hash + Default {}
impl<T: Clone + Eq + Hash + Default> TraceI for T {}

pub trait Game: Sized + Clone + 'static {
    type State: Clone + 'static;
    type Action: ActionI + 'static;
    type Observation: Clone + 'static;
    type Trace: TraceI + 'static;

    // Encode/decode world state
    fn encode(&self) -> Self::State;
    fn decode(state: &Self::State) -> Self;

    // Public trace + perspective helpers
    fn trace(&self, player: Player) -> Self::Trace;
    fn perspective(&self, trace: Self::Trace) -> Player;

    // Local dynamics
    fn active_player(&self) -> Player;
    fn available_actions(&self) -> Vec<Self::Action>;
    fn observation(&self, player: Player) -> Self::Observation;
    fn play(&self, action: &Self::Action) -> Self;
    fn is_over(&self) -> bool;
    fn evaluate(&self) -> Reward; // a quick static eval

    // Pluggable sampler to seed subgames
    fn sample_positions(observation_history: &Self::Trace) -> Vec<Self>;
}

// ---------- Policy ----------
#[derive(Clone)]
pub struct Policy<A: ActionI> {
    pub multiplier: i8, // +1 for maximizing player, -1 for minimizing (in zero-sum CFV space)
    pub actions: Vec<A>,
    pub expectations: Vec<Reward>,
    pub expansions: Vec<usize>,
    pub acc_regrets: Vec<Reward>,
    pub stable: Vec<bool>,
    pub updates: usize,
}

impl<A: ActionI> Policy<A> {
    pub fn from_actions(actions: Vec<A>, multiplier: i8) -> Self {
        let n = actions.len();
        Policy {
            multiplier,
            actions,
            expectations: vec![0.0; n],
            expansions: vec![0; n],
            acc_regrets: vec![1e-12; n],
            stable: vec![false; n],
            updates: 0,
        }
    }

    pub fn from_rewards(items: Vec<(A, Reward)>, player: Player) -> Self {
        let (actions, expectations): (Vec<A>, Vec<Reward>) = items.into_iter().unzip();
        let n = expectations.len();
        Policy {
            multiplier: match player { Player::P1 => 1, Player::P2 => -1 },
            actions,
            expectations,
            expansions: vec![0; n],
            acc_regrets: vec![1e-12; n],
            stable: vec![false; n],
            updates: 0,
        }
    }

    fn quality(&self, idx: usize) -> f64 {
        // very light UCB/PUCT-style score using expansions + expectations
        let n = self.expansions.iter().sum::<usize>().max(1) as f64;
        let v = self.expectations[idx];
        let c = 1.25;
        v + c * ((n.ln() / (self.expansions[idx].max(1) as f64)).sqrt())
    }

    fn puct(&self) -> Vec<Probability> {
        if self.actions.is_empty() { return vec![]; }
        let mut best = 0usize;
        for i in 1..self.actions.len() {
            if self.quality(i) > self.quality(best) { best = i; }
        }
        let mut out = vec![0.0; self.actions.len()];
        out[best] = 1.0;
        out
    }

    fn exploit_policy(&self) -> Vec<Probability> {
        if self.actions.is_empty() { return vec![]; }
        let sum: f64 = self.acc_regrets.iter().sum();
        if sum <= 0.0 || !sum.is_finite() {
            // uniform
            let p = 1.0 / (self.actions.len() as f64);
            return vec![p; self.actions.len()];
        }
        self.acc_regrets.iter().map(|r| r / sum).collect()
    }

    fn exploration_policy(&self) -> Vec<Probability> {
        // simple 50/50 between puct single-arm and exploit mix
        let puct = self.puct();
        let exploit = self.exploit_policy();
        if puct.is_empty() { return exploit; }
        puct.iter().zip(exploit.iter()).map(|(a,b)| 0.5*a + 0.5*b).collect()
    }

    fn sample_from(&self, probs: &[Probability]) -> A {
        let net: f64 = probs.iter().sum();
        let mut rng = thread_rng();
        if probs.is_empty() {
            panic!("empty policy actions");
        }
        if net <= 0.0 { return self.actions.iter().choose(&mut rng).unwrap().clone(); }
        let weights: Vec<f64> = probs.iter().map(|p| if *p <= 0.0 { 0.0 } else { p / net }).collect();
        let dist = WeightedIndex::new(weights).unwrap();
        let idx = dist.sample(&mut rng);
        self.actions[idx].clone()
    }

    pub fn explore(&self) -> A { self.sample_from(&self.exploration_policy()) }
    pub fn exploit(&self) -> A { self.sample_from(&self.exploit_policy()) }

    pub fn purified(&self) -> A {
        // choose among top-K by exploit prob with tie-breaking random among equals
        let probs = self.exploit_policy();
        let mut idxs: Vec<(usize, f64)> = probs.iter().cloned().enumerate().collect();
        idxs.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        let k = idxs.iter().take(MAX_SUPPORT.max(1)).map(|(i,_)| *i).collect::<Vec<_>>();
        let mut rng = thread_rng();
        self.actions[*k.iter().choose(&mut rng).unwrap()].clone()
    }

    pub fn set_expectation(&mut self, a: &A, v: Reward) {
        let idx = self.actions.iter().position(|x| x == a).unwrap();
        self.expectations[idx] = v;
    }

    pub fn add_expansion(&mut self, a: &A) {
        let idx = self.actions.iter().position(|x| x == a).unwrap();
        self.expansions[idx] += 1;
    }

    pub fn expectation(&self) -> Reward {
        if self.expectations.is_empty() { return 0.0; }
        self.expectations.iter().sum::<f64>() / (self.expectations.len() as f64)
    }

    pub fn p_exploit(&self, a: &A) -> Probability {
        let idx = self.actions.iter().position(|x| x == a).unwrap();
        let sum: f64 = self.acc_regrets.iter().sum();
        if sum <= 0.0 { return 0.0; }
        self.acc_regrets[idx] / sum
    }

    pub fn update(&mut self) {
        // last-iterate CFR+-ish push of positive advantages vs a simple baseline
        let n = self.expectations.len() as f64;
        if n <= 0.0 { return; }
        let baseline = self.expectations.iter().sum::<f64>() / n;
        let mult = self.multiplier as f64;
        let eps = 1e-12;

        for i in 0..self.expectations.len() {
            let adv = mult * (self.expectations[i] - baseline);
            if adv > 0.0 {
                self.acc_regrets[i] += adv;
            }
            if !self.acc_regrets[i].is_finite() || self.acc_regrets[i] <= 0.0 {
                self.acc_regrets[i] = eps;
            }
        }

        // mark current best as stable (cheap purification hint)
        if !self.acc_regrets.is_empty() {
            let best = (0..self.acc_regrets.len())
                .max_by(|&i,&j| self.acc_regrets[i].partial_cmp(&self.acc_regrets[j]).unwrap_or(Ordering::Equal));
            if let Some(i) = best { self.stable[i] = true; }
        }

        self.updates += 1;
    }
}

// ---------- Info (an infoset) ----------
pub type InfoPtr<A, T> = Rc<RefCell<Info<A, T>>>;

pub struct Info<A: ActionI, T: TraceI> {
    pub policy: Policy<A>,
    pub trace: T,
    pub player: Player,
    pub reach: HashMap<Player, Probability>,
    pub visited: bool,

    // Successor metadata for gift() and filter()
    pub succ_traces: Vec<Option<T>>,
    pub succ_ptrs: Vec<Option<InfoPtr<A, T>>>,
    pub succ_alt: Vec<Option<Reward>>,

    gift_cached: RefCell<Option<Reward>>,
}

impl<A: ActionI, T: TraceI> Info<A, T> {
    pub fn new(actions: Vec<A>, trace: T, player: Player) -> Self {
        let n = actions.len();
        Info {
            policy: Policy::from_actions(actions, match player { Player::P1 => 1, Player::P2 => -1 }),
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

// ---------- Resolve gadget ----------
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ResolveActions { SKIP, ENTER }

// ---------- History ----------
pub enum History<G: Game> {
    Terminal { payoff: Reward },
    New { state: Box<G::State> },
    Visited { state: Box<G::State>, reach: HashMap<Player, Probability> },
    Expanded { info: InfoPtr<G::Action, G::Trace>, reach: HashMap<Player, Probability>, children: Vec<(G::Action, History<G>)>, player: Player },
    Augmented { info: Info<usize, G::Trace>, resolver: Policy<ResolveActions>, alt: Reward, prior_probability: Probability, children: Vec<(usize, History<G>)> },
    SubgameRoot { info: Info<usize, G::Trace>, children: Vec<(usize, History<G>)> },
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
            History::Augmented { info, resolver, alt, prior_probability, children } => History::Augmented {
                info: Info {
                    policy: info.policy.clone(),
                    trace: info.trace.clone(),
                    player: info.player,
                    reach: info.reach.clone(),
                    visited: info.visited,
                    succ_traces: info.succ_traces.clone(),
                    succ_ptrs: info.succ_ptrs.clone(),
                    succ_alt: info.succ_alt.clone(),
                    gift_cached: RefCell::new(*info.gift_cached.borrow()),
                },
                resolver: resolver.clone(),
                alt: *alt,
                prior_probability: *prior_probability,
                children: children.clone(),
            },
            History::SubgameRoot { info, children } => History::SubgameRoot {
                info: Info {
                    policy: info.policy.clone(),
                    trace: info.trace.clone(),
                    player: info.player,
                    reach: info.reach.clone(),
                    visited: info.visited,
                    succ_traces: info.succ_traces.clone(),
                    succ_ptrs: info.succ_ptrs.clone(),
                    succ_alt: info.succ_alt.clone(),
                    gift_cached: RefCell::new(*info.gift_cached.borrow()),
                },
                children: children.clone(),
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
            History::Augmented { alt, .. } => *alt,
            History::SubgameRoot { .. } => 0.0,
        }
    }

    pub fn player(&self) -> Player {
        match self {
            History::Terminal { .. } => panic!("terminal has no player"),
            History::New { state } | History::Visited { state, .. } => G::decode(state).active_player(),
            History::Expanded { info, .. } => info.borrow().player,
            History::Augmented { info, .. } | History::SubgameRoot { info, .. } => info.player,
        }
    }

    pub fn trace(&self) -> G::Trace {
        match self {
            History::Terminal { .. } => Default::default(),
            History::New { state } | History::Visited { state, .. } => {
                let g = G::decode(state);
                g.trace(g.active_player())
            }
            History::Expanded { info, .. } => info.borrow().trace.clone(),
            History::Augmented { info, .. } | History::SubgameRoot { info, .. } => info.trace.clone(),
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
                let info = Info::new(actions.clone(), this_trace.clone(), player);
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

    pub fn subgame_root(j0: HashMap<G::Trace, (Probability, Reward, Vec<Self>)>, player: Player) -> Self {
        // Prior for each J node = given probability (already normalized upstream if desired)
        let mut items: Vec<(usize, History<G>)> = Vec::new();
        for (_tr, (_pp, alt, entries)) in j0.into_iter() {
            // Create an augmented gadget that mixes SKIP (alt) vs ENTER (children)
            let m = entries.len().max(1);
            let info = Info::new((0..m).collect::<Vec<usize>>(), Default::default(), player);
            let resolver = Policy::from_rewards(vec![
                (ResolveActions::SKIP, alt),
                (ResolveActions::ENTER, 0.0),
            ], player.other());

            let augmented = History::Augmented {
                info,
                resolver,
                alt,
                prior_probability: 1.0, // normalized later if you use it
                children: entries.into_iter().enumerate().collect(),
            };
            items.push((items.len(), augmented));
        }
        let info = Info::new((0..items.len()).collect::<Vec<usize>>(), Default::default(), player);
        History::SubgameRoot { info, children: items }
    }
}

// ---------- Solver ----------
pub struct Obscuro<G: Game> {
    pub expectation: Reward,
    pub info_sets: HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>,
    pub subgame_root: Option<History<G>>,
    pub start_time: SystemTime,
    pub active_player: Player,
}

impl<G: Game> Default for Obscuro<G> {
    fn default() -> Self {
        Self {
            expectation: 0.0,
            info_sets: HashMap::new(),
            subgame_root: None,
            start_time: SystemTime::now(),
            active_player: Player::P1,
        }
    }
}

impl<G: Game> Obscuro<G> {
    fn get_j0(subgame_root: &mut Option<History<G>>) -> Vec<&mut History<G>> {
        if let Some(History::SubgameRoot { children, .. }) = subgame_root {
            return children.iter_mut().map(|(_, h)| h).collect();
        }
        vec![]
    }

    fn get_pmax(&self) -> Probability {
        if let Some(History::SubgameRoot { children, .. }) = &self.subgame_root {
            return children.iter().map(|(_, h)| {
                if let History::Augmented { resolver, .. } = h {
                    resolver.p_exploit(&ResolveActions::ENTER)
                } else { 0.0 }
            }).fold(0.0, f64::max);
        }
        0.0
    }

    pub fn filter(&mut self, target: G::Trace) -> Vec<History<G>> {
        let mut out = Vec::new();
        if let Some(root) = &self.subgame_root {
            Self::collect_matching::<G>(root, &target, &mut out);
        }
        out
    }

    fn collect_matching<G2: Game>(h: &History<G2>, want: &G2::Trace, out: &mut Vec<History<G2>>) {
        match h {
            History::Terminal { .. } => {}
            History::New { state } | History::Visited { state, .. } => {
                let g = G2::decode(state);
                let here = g.trace(g.active_player());
                if &here == want {
                    out.push(History::Visited { state: state.clone(), reach: HashMap::new() });
                }
            }
            History::Expanded { info, children, .. } => {
                if &info.borrow().trace == want {
                    out.push(h.clone());
                }
                for (_, ch) in children {
                    Self::collect_matching::<G2>(ch, want, out);
                }
            }
            History::Augmented { children, .. } | History::SubgameRoot { children, .. } => {
                for (_, ch) in children {
                    Self::collect_matching::<G2>(ch, want, out);
                }
            }
        }
    }

    pub fn construct_subgame(&mut self, hist: G::Trace, player: Player) {
        let other = player.other();
        let mut positions: Vec<G> = Game::sample_positions(&hist);

        let mut histories = self.filter(hist.clone());
        let mut data_count = histories.len();

        // seed j0 from current tree
        let mut j0: HashMap<G::Trace, (Probability, Reward, Vec<History<G>>)> = HashMap::new();
        for h in histories.drain(..) {
            let opp_trace = match &h {
                History::New { state } | History::Visited { state, .. } => {
                    let g = G::decode(state);
                    g.trace(other)
                }
                History::Expanded { info, .. } => info.borrow().trace.clone(),
                _ => continue,
            };

            let alt = if let Some(rc) = self.info_sets.get(&opp_trace) {
                let j = rc.borrow();
                j.policy.expectation() - j.gift()
            } else { h.payoff() };

            j0.entry(opp_trace).and_modify(|e| {
                e.2.push(h.clone());
            }).or_insert((1.0, alt, vec![h]));
        }

        // pad with samples until minimum size
        while data_count < MIN_INFO_SIZE.min(positions.len()) {
            if let Some(g) = positions.pop() {
                let s = History::New { state: Box::new(g.encode()) };
                let opp_trace = g.trace(other);
                let alt = g.evaluate();
                j0.entry(opp_trace).and_modify(|e| {
                    e.2.push(s.clone());
                }).or_insert((1.0, alt, vec![s]));
                data_count += 1;
            } else { break; }
        }

        self.subgame_root = Some(History::subgame_root(j0, player));
    }

    fn make_utilities(h: &mut History<G>, optimizing_player: Player, reach_prob: Probability) -> Reward {
        match h {
            History::Terminal { payoff } => *payoff,

            History::New { state } => {
                let mut v = History::Visited { state: state.clone(), reach: HashMap::new() };
                Self::make_utilities(&mut v, optimizing_player, reach_prob)
            }

            History::Visited { .. } => {
                // Expand lazily only when necessary (caller should expand before calling utilities normally)
                0.0
            }

            History::Expanded { info, player, children, .. } => {
                let mut local = 0.0;
                for (a, child) in children.iter_mut() {
                    let p = info.borrow().policy.p_exploit(a);
                    if *player == optimizing_player || p > 0.0 {
                        let v = Self::make_utilities(child, optimizing_player, reach_prob * p);
                        local += p * v;
                        info.borrow_mut().add_counterfactuals(a.clone(), v);
                    }
                }
                info.borrow_mut().update();
                local
            }

            History::Augmented { info, resolver, alt, children, .. } => {
                // ENTER branch: value under info's mix over children
                let mut enter_v = 0.0;
                for (idx, child) in children.iter_mut() {
                    let p_idx = info.policy.p_exploit(idx);
                    if p_idx > 0.0 {
                        let v = Self::make_utilities(child, optimizing_player, reach_prob * p_idx);
                        enter_v += p_idx * v;
                    }
                }
                resolver.set_expectation(&ResolveActions::ENTER, enter_v);
                resolver.set_expectation(&ResolveActions::SKIP, *alt);
                resolver.update();
                let p_enter = resolver.p_exploit(&ResolveActions::ENTER);
                (1.0 - p_enter) * *alt + p_enter * enter_v
            }

            History::SubgameRoot { info, children } => {
                let mut v = 0.0;
                for (idx, child) in children.iter_mut() {
                    let p = info.policy.p_exploit(idx);
                    if p > 0.0 {
                        let vv = Self::make_utilities(child, optimizing_player, reach_prob * p);
                        // update root expectations per child index
                        let i = *idx as usize;
                        if i < info.policy.expectations.len() {
                            info.policy.expectations[i] = vv;
                        }
                        v += p * vv;
                    }
                }
                info.update();
                v
            }
        }
    }

    fn choose_action_from_root(&self) -> Option<G::Action> {
        // Walk SubgameRoot -> chosen Augmented child -> first Expanded node's policy, then purified action
        let root = match &self.subgame_root {
            Some(h) => h,
            None => return None,
        };
        if let History::SubgameRoot { info, children } = root {
            // choose highest prob child
            let mut best = None::<(usize, f64)>;
            for (idx, _) in children.iter() {
                let p = info.policy.p_exploit(idx);
                match best {
                    None => best = Some((*idx, p)),
                    Some((_, bp)) if p > bp => best = Some((*idx, p)),
                    _ => {}
                }
            }
            if let Some((idx, _)) = best {
                if let Some((_, child)) = children.iter().find(|(i, _)| *i == idx) {
                    if let History::Augmented { children, .. } = child {
                        for (_, h) in children {
                            if let History::Expanded { info, .. } = h {
                                return Some(info.borrow().policy.purified());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn make_move(&mut self, observation: G::Trace, player: Player) -> G::Action {
        println!("Making move: {:?}", player);
        self.start_time = SystemTime::now();
        self.active_player = player;

        self.construct_subgame(observation.clone(), player);

        // very lightweight loop: expand each J0 member once, then evaluate gadgets
        while self.start_time.elapsed().unwrap_or(Duration::from_secs(0)) < Duration::from_secs(SOLVE_TIME_SECS) {
            // Split-borrow self so we can mutate two fields without tripping the borrow checker
            let (subgame_root_ref, info_sets_ref) = {
                let Self{ subgame_root, info_sets, .. } = self;
                (subgame_root as *mut Option<History<G>>, info_sets as *mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>)
            };

            // SAFETY: we never alias the same field; they are distinct
            let j0: Vec<*mut History<G>> = unsafe {
                let subgame_root_mut = &mut *subgame_root_ref;
                Obscuro::<G>::get_j0(subgame_root_mut).into_iter().map(|h| h as *mut History<G>).collect()
            };

            for h_ptr in j0 {
                unsafe {
                    let h = &mut *h_ptr;
                    if let History::Augmented { children, .. } = h {
                        for (_, child) in children.iter_mut() {
                            if let History::Visited { .. } = child {
                                let info_sets_mut = &mut *info_sets_ref;
                                child.expand(info_sets_mut);
                            }
                        }
                    }
                }
            }

            // Evaluate utilities from root
            if let Some(root) = &mut self.subgame_root {
                let _ = Self::make_utilities(root, player, 1.0);
            } else { break; }
        }

        // return purified best from chosen expanded node; if missing, fall back to random on any infoset for player
        if let Some(a) = self.choose_action_from_root() { return a; }

        // Fallback: pick an action from any infoset for the player
        for (_t, rc) in self.info_sets.iter() {
            let info = rc.borrow();
            if info.player == player && !info.policy.actions.is_empty() {
                return info.policy.purified();
            }
        }

        panic!("No action available");
    }

    // Optional helper to inspect current closure (stub)
    pub fn info_closure(&self) -> Vec<TinyInfo<G::Action, G::Trace>> {
        self.info_sets.iter().map(|(t, rc)| {
            let i = rc.borrow();
            TinyInfo {
                trace: t.clone(),
                player: i.player,
                expectation: i.policy.expectation(),
                _phantom: PhantomData,
            }
        }).collect()
    }
}

pub struct TinyInfo<A, T> { pub trace: T, pub player: Player, pub expectation: Reward, _phantom: PhantomData<A>}

// ---------- Demo Game: Rock-Paper-Scissors (sequential, perfect info) ----------

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum RpsAction { Rock, Paper, Scissors }

impl std::fmt::Debug for RpsAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpsAction::Rock => write!(f, "Rock"),
            RpsAction::Paper => write!(f, "Paper"),
            RpsAction::Scissors => write!(f, "Scissors"),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Default)]
pub struct RpsTrace(pub u8); // 0 = start, 1 = after P1 move, 2 = terminal

#[derive(Clone)]
pub struct Rps {
    p1: Option<RpsAction>,
    p2: Option<RpsAction>,
    to_move: Player,
}

impl Rps {
    pub fn new() -> Self {
        Self { p1: None, p2: None, to_move: Player::P1 }
    }
}

impl Game for Rps {
    type State = Self;
    type Action = RpsAction;
    type Observation = (Option<RpsAction>, Option<RpsAction>);
    type Trace = RpsTrace;

    fn encode(&self) -> Self::State { self.clone() }
    fn decode(state: &Self::State) -> Self { state.clone() }

    fn trace(&self, _player: Player) -> Self::Trace {
        let stage = match (self.p1.is_some(), self.p2.is_some()) {
            (false, _) => 0,
            (true, false) => 1,
            (true, true) => 2,
        };
        RpsTrace(stage)
    }
    fn perspective(&self, _trace: Self::Trace) -> Player { self.to_move }

    fn active_player(&self) -> Player { self.to_move }

    fn available_actions(&self) -> Vec<Self::Action> {
        if self.is_over() { return vec![]; }
        vec![RpsAction::Rock, RpsAction::Paper, RpsAction::Scissors]
    }

    fn observation(&self, _player: Player) -> Self::Observation {
        (self.p1.clone(), self.p2.clone())
    }

    fn play(&self, action: &Self::Action) -> Self {
        let mut s = self.clone();
        match self.to_move {
            Player::P1 => {
                s.p1 = Some(action.clone());
                s.to_move = Player::P2;
            }
            Player::P2 => {
                s.p2 = Some(action.clone());
                s.to_move = Player::P1; // irrelevant after terminal
            }
        }
        s
    }

    fn is_over(&self) -> bool {
        self.p1.is_some() && self.p2.is_some()
    }

    fn evaluate(&self) -> Reward {
        // Terminal payoff for P1; non-terminal = 0.0 (neutral heuristic)
        if let (Some(a), Some(b)) = (&self.p1, &self.p2) {
            if a == b { return 0.0; }
            // Rock beats Scissors, Scissors beats Paper, Paper beats Rock
            let p1_wins = matches!((a, b),
                (RpsAction::Rock, RpsAction::Scissors) |
                (RpsAction::Scissors, RpsAction::Paper) |
                (RpsAction::Paper, RpsAction::Rock)
            );
            let winner_used_rock =
                (p1_wins && matches!(a, RpsAction::Rock)) ||
                (!p1_wins && matches!(b, RpsAction::Rock));
            let mag = if winner_used_rock { 5.0 } else { 1.0 };
            return if p1_wins { mag } else { -mag };
        }
        0.0
    }

    fn sample_positions(_observation_history: &Self::Trace) -> Vec<Self> {
        // Produce a bag of states at different stages to seed subgames.
        // We'll include initial states and "after P1 move" states for variety.
        let mut out = Vec::with_capacity(256);
        // All-empty
        for _ in 0..64 {
            out.push(Rps::new());
        }
        // P1 pre-commits a move (P2 to move)
        for _ in 0..64 {
            out.push(Rps { p1: Some(RpsAction::Rock), p2: None, to_move: Player::P2 });
            out.push(Rps { p1: Some(RpsAction::Paper), p2: None, to_move: Player::P2 });
            out.push(Rps { p1: Some(RpsAction::Scissors), p2: None, to_move: Player::P2 });
        }
        out
    }
}

// ---------- Tests / demo scaffolding ----------
pub(crate) fn main_obscoro() {
    // Tiny smoke test: construct a game and ask Obscuro for a move for P1 at the root
    let mut solver: Obscuro<Rps> = Obscuro::default();
    let obs = Rps::new().trace(Player::P1);
    let action = solver.make_move(obs, Player::P1);
    println!("Chosen action for P1: {:?}", action);
}

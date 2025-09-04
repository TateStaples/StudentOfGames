use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::time::{Duration, SystemTime};
use crate::obscuro::utils::*;
use crate::obscuro::history::*;
use crate::obscuro::info::*;
use crate::obscuro::obscuro::Node::{Augmented, Internal, Root};
use crate::obscuro::obscuro::ResolveActions::{ENTER, SKIP};
use crate::obscuro::policy::Policy;

// TODO: where is the 

// ---------- Resolve gadget ----------
struct SubgameRoot<G: Game> { policy: Policy<usize>, children: Vec<ResolverGadget<G>> }
impl<G: Game> SubgameRoot<G> {
    /// Create a new subgame root from 2-cover, all the infostates the other player believes we could be in 
    pub fn new(j0: HashMap<G::Trace, (Probability, Reward, Vec<History<G>>)>, player: Player) -> Self {
        // Prior for each J node = given probability (already normalized upstream if desired)
        let mut items: Vec<ResolverGadget<G>> = Vec::new();
        for (trace, (_pp, alt, entries)) in j0.into_iter() {
            // Create an augmented gadget that mixes SKIP (alt) vs ENTER (children)
            let info = Info::from_policy(
                Policy::from_rewards(entries.iter().map(|x| x.reach_prob(player)).enumerate().collect(), player),
                trace, Player::Random,
            );
            let resolver = Policy::from_rewards(vec![
                (SKIP, alt),
                (ENTER, 0.0),
            ], player.other());

            let augmented = ResolverGadget {
                info,
                resolver,
                alt,
                prior_probability: 1.0, // normalized later if you use it
                children: entries,
            };
            items.push(augmented);
        }
        let root_policy = Policy::from_rewards(vec![(0, 0.0)], player);  // FIXME:
        // debug_assert!(root_policy.actions.len() == j0.len());
        SubgameRoot { policy: root_policy, children: items }
    }
}
/// Safe-Resolving Gadget to determine whether opponent would enter this subgame. TODO: figure out default
struct ResolverGadget<G: Game> {
    info: Info<usize, G::Trace>,  // Info policy showing the probability distribution of reach odds for each child in this opp. infoset
    resolver: Policy<ResolveActions>,
    alt: Reward,
    prior_probability: Probability,
    children: Vec<History<G>>
}
type PreResolver<G> = (Probability, Reward, Vec<History<G>>);
/// Full Solving Tree
enum Node<G: Game> {
    Root(SubgameRoot<G>),
    Augmented(ResolverGadget<G>),
    Internal(History<G>)
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ResolveActions { SKIP, ENTER }
// ---------- Solver ----------
pub struct Obscuro<G: Game> {
    pub expectation: Reward,
    pub info_sets: HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>,
    pub subgame_root: SubgameRoot<G>,  // TODO: think more about how to structure this at the start of the game
    pub start_time: SystemTime,
    pub active_player: Player,
}

impl<G: Game> Default for Obscuro<G> {
    fn default() -> Self {
        let game = G::new();
        let root = SubgameRoot{
            policy: Policy::from_rewards(vec![(0, 1.0)], game.active_player()),
            children: vec![
                (ResolverGadget { 
                    info: Info::from_policy(Policy::from_actions(vec![0], game.active_player()), Default::default(), game.active_player()),
                    resolver: Policy::from_rewards(vec![(SKIP, 0.0), (ENTER, Reward::MAX/2.0)], game.active_player().other()),  
                    alt: Player::best_value(game.active_player()), 
                    prior_probability: 1.0, 
                    children: vec![History::new(game.encode())]
                })
            ],
        };
        Self {
            expectation: 0.0,
            info_sets: HashMap::new(),
            subgame_root: root,
            start_time: SystemTime::now(),
            active_player: Player::P1,
        }
    }
}


impl<G: Game> Obscuro<G> {
    /// Get the Augmented Subgames that need to be explored. One per opposing InfoState
    fn get_j0(subgame_root: &mut SubgameRoot<G>) -> Vec<&mut ResolverGadget<G>> {
        subgame_root.children.iter_mut().map(|(h)| h).collect()
    }

    /// How likely is it that the opponent would ever enter this subgame
    fn get_pmax(&self) -> Probability {
        self.subgame_root.children.iter().map(|h| {
            let ResolverGadget { resolver, .. } = h;
            resolver.p_exploit(&ENTER)
        }).fold(0.0, f64::max)
    }
    
    fn drain_root(&mut self) -> Vec<ResolverGadget<G>> {
        // Mem swap out the vec from root
        // let mut j0 = Self::get_j0(&mut self.subgame_root);
        let mut j0 = Vec::new();
        std::mem::swap(&mut j0, &mut self.subgame_root.children);
        j0
    }

    /// Update "now" in the solver
    /// TODO: figure out the initial construction
    pub fn construct_subgame(&mut self, hist: G::Trace, player: Player) {  // How does ord work for multiple players?
        type PreResolver<G> = (Probability, Reward, Vec<History<G>>);
        let other = player.other();
        // Find all root histories
        // Filter down to the second cover of the trace -> split by opponent infostate (they are kinda doing it by post action infostate)
        // TODO: integrate prior_probs & gifts for the results of k2_cover
        let covered = Self::k_cover();
        let mut positions: HashMap<G::Trace, PreResolver<G>> = HashMap::new();  // Resolve node has V_alt,
        // Augment with additional matching traces
        let mut data_count = positions.len();
        let mut new_positions = G::sample_position(hist);
        while data_count < MIN_INFO_SIZE.min(positions.len()) {
            if let Some(g) = new_positions.next() {
                let s = History::New { state: Box::new(g.encode()) };
                let opp_trace = g.trace(other);
                let alt = g.evaluate();
                let resolver_info = (1.0, alt, vec![s.clone()]);
                positions.entry(opp_trace).and_modify(|e| {
                    e.2.push(s);
                }).or_insert(resolver_info);
                data_count += 1;
            } else { break; }
        }
        // Initialize the Resolver Nodes: alt, chance node with Resolver policy

        // Add Root with opp. policy to choose their infostate
        let root = SubgameRoot::new(positions, player);
        self.subgame_root = root;
        // At start of game -> J0 = single Augmented with 1 contained history

        // let mut histories = self.filter(hist.clone());
        // P := all positions consistent with trace
        // I := all infosets consistent with trace
        // J0 := all opponent infosets consistent under second cover of I
        // Root is opponent decision node for which infoset in J0 they want to be at
        // J \in J0 is random node to sample history from
        // let mut data_count = histories.len();
        //
        // // seed j0 from current tree
        // let mut j0: HashMap<G::Trace, (Probability, Reward, Vec<History<G>>)> = HashMap::new();
        // for h in histories.drain() {
        //     let opp_trace = match &h {
        //         History::New { state } | History::Visited { state, .. } => {
        //             let g = G::decode(state);
        //             g.trace(other)
        //         }
        //         History::Expanded { info, .. } => info.borrow().trace.clone(),
        //         _ => continue,
        //     };
        //
        //     let alt = if let Some(rc) = self.info_sets.get(&opp_trace) {
        //         let j = rc.borrow();
        //         j.policy.expectation() - j.gift()
        //     } else { h.payoff() };
        //
        //     j0.entry(opp_trace).and_modify(|e| {
        //         e.2.push(h.clone());
        //     }).or_insert((1.0, alt, vec![h]));
        // }

        // pad with samples until minimum size

        // println!("data_count: {}, j0 size: {}", data_count, j0.len());
        // self.subgame_root = SubgameRoot::new(j0, player);
    }

    fn k_cover(root_histories: Vec<History<G>>, hist: HashSet<G::Trace>, k: u8) -> Vec<History<G>> {
        // Find all nodes matching this trace
        let (mut covered, rest): (Vec<_>, Vec<_>) = root_histories.into_iter().partition(|h| hist.contains(&h.trace())).collect();
        // if k > 1, find all nodes in k-1_cover of all other player traces
        if k > 1 {
            covered.extend(Self::k_cover(rest, covered.iter().map(|&h| h.trace()), k-1));  // FIXME: should be the other player's trace
        }
        covered
    }

    fn make_utilities(h: &mut Node<G>, optimizing_player: Player, reach_prob: Probability) -> Reward {
        match h {  
            Internal(h) => {Self::make_utilities_hist(h, optimizing_player, reach_prob)}
            Augmented(ResolverGadget{ info, resolver, alt, children, .. }) => {
                // ENTER branch: value under info's mix over children
                let mut enter_v = 0.0;
                for (idx, child) in children.iter_mut().enumerate() {
                    let p_idx = info.policy.p_exploit(&idx);
                    if p_idx > 0.0 {
                        let v = Self::make_utilities(Internal(child), optimizing_player, reach_prob * p_idx);
                        enter_v += p_idx * v;
                    }
                }
                resolver.set_expectation(&ENTER, enter_v);
                resolver.set_expectation(&SKIP, *alt);
                resolver.update();
                let p_enter = resolver.p_exploit(&ENTER);
                (1.0 - p_enter) * *alt + p_enter * enter_v
            }

            Root(SubgameRoot { policy, children }) => {
                let mut v = 0.0;
                for (idx, child) in children.iter_mut().enumerate() {
                    let p = policy.p_exploit(idx);
                    if p > 0.0 {
                        let vv = Self::make_utilities(Augmented(child), optimizing_player, reach_prob * p);
                        // update root expectations per child index
                        let i = idx;
                        if i < policy.expectations.len() {
                            policy.expectations[i] = vv;
                        }
                        v += p * vv;
                    }
                }
                policy.update();
                v
            }
        }
    }

    fn make_utilities_hist(h: &mut History<G>, optimizing_player: Player, reach_prob: Probability) -> Reward {
        match h {
            History::Terminal { payoff } => *payoff,

            History::New { state } => {
                let mut v = History::Visited { state: state.clone(), reach: HashMap::new() };
                Self::make_utilities(&mut Internal(v), optimizing_player, reach_prob)
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
        }
    }
    fn choose_action_from_root(&self) -> Option<G::Action> {
        todo!()
        // Walk SubgameRoot -> chosen Augmented child -> first Expanded node's policy, then purified action

        // if let SubgameRoot { info, children } = self.subgame_root {
        // choose highest prob child
        // let children = self.subgame_root.children;
        // let info = self.subgame_root.info;
        // let mut best = None::<(usize, f64)>;
        //
        // for (idx, _) in children.iter() {
        //     let p = info.policy.p_exploit(idx);
        //     match best {
        //         None => best = Some((*idx, p)),
        //         Some((_, bp)) if p > bp => best = Some((*idx, p)),
        //         _ => {}
        //     }
        // }
        // if let Some((idx, _)) = best {
        //     if let Some((_, child)) = children.iter().find(|(i, _)| *i == idx) {
        //         if let ResolverGadget { children, .. } = child {
        //             for (_, h) in children {
        //                 if let History::Expanded { info, .. } = h {
        //                     return Some(info.borrow().policy.purified());
        //                 }
        //             }
        //         }
        //     }
        // }
    }

    
    pub fn make_move(&mut self, observation: G::Trace, player: Player) -> G::Action {
        println!("Making move: {:?}, {:?}", player, observation);
        self.start_time = SystemTime::now();
        self.active_player = player;

        self.construct_subgame(observation.clone(), player);

        // very lightweight loop: expand each J0 member once, then evaluate gadgets
        while self.start_time.elapsed().unwrap_or(Duration::from_secs(0)) < Duration::from_secs(SOLVE_TIME_SECS) {
            // println!("LOOP!");
            // Split-borrow self so we can mutate two fields without tripping the borrow checker
            let (subgame_root_mut, info_sets_mut) = {
                let Self{ subgame_root, info_sets, .. } = self;
                (subgame_root as &mut SubgameRoot<G>, info_sets as &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>)
            };
            
            

            // Evaluate utilities from root
            Self::make_utilities(Root(subgame_root_mut), player, 1.0);
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

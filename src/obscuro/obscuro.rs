use crate::obscuro::history::*;
use crate::obscuro::info::*;
use crate::obscuro::obscuro::ResolveActions::{ENTER, SKIP};
use crate::obscuro::policy::Policy;
use crate::obscuro::utils::*;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::time::{Duration, SystemTime};

// TODO: where is the

// ---------- Resolve gadget ----------
struct SubgameRoot<G: Game> {
    policy: Policy<usize>,
    children: Vec<ResolverGadget<G>>,
}
impl<G: Game> SubgameRoot<G> {
    /// Create a new subgame root from 2-cover, all the infostates the other player believes we could be in
    pub fn new(
        j0: HashMap<G::Trace, PreResolver<G>>,
        player: Player,
    ) -> Self {
        // Prior for each J node = given probability (already normalized upstream if desired)
        let mut items: Vec<ResolverGadget<G>> = Vec::new();
        for (trace, (_pp, alt, entries)) in j0.into_iter() {
            // Create an augmented gadget that mixes SKIP (alt) vs ENTER (children)
            let info = Info::from_policy(
                Policy::from_rewards(
                    entries
                        .iter()
                        .map(|x| x.reach_prob(player))
                        .enumerate()
                        .collect(),
                    player,
                ),
                trace,
                Player::Random,
            );
            let resolver = Policy::from_rewards(vec![(SKIP, alt), (ENTER, 0.0)], player.other());

            let augmented = ResolverGadget {
                info,
                resolver,
                alt,
                prior_probability: 1.0, // normalized later if you use it
                children: entries,
            };
            items.push(augmented);
        }
        let root_policy = Policy::from_rewards(vec![(0, 0.0)], player); // FIXME:
                                                                        // debug_assert!(root_policy.actions.len() == j0.len());
        SubgameRoot {
            policy: root_policy,
            children: items,
        }
    }
}
/// Safe-Resolving Gadget to determine whether opponent would enter this subgame. TODO: figure out default
struct ResolverGadget<G: Game> {
    info: Info<usize, G::Trace>, // Info policy showing the probability distribution of reach odds for each child in this opp. infoset
    resolver: Policy<ResolveActions>,
    alt: Reward,
    prior_probability: Probability,
    children: Vec<History<G>>,
}
type PreResolver<G> = (Probability, Reward, Vec<History<G>>);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ResolveActions {
    SKIP,
    ENTER,
}
// ---------- Solver ----------
pub struct Obscuro<G: Game> {
    pub expectation: Reward,
    pub info_sets: HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>,
    pub subgame_root: SubgameRoot<G>, // TODO: think more about how to structure this at the start of the game
    pub start_time: SystemTime,
    pub active_player: Player,
}

impl<G: Game> Default for Obscuro<G> {
    fn default() -> Self {
        let game = G::new();
        let root = SubgameRoot {
            policy: Policy::from_rewards(vec![(0, 1.0)], game.active_player()),
            children: vec![
                (ResolverGadget {
                    info: Info::from_policy(
                        Policy::from_actions(vec![0], game.active_player()),
                        Default::default(),
                        game.active_player(),
                    ),
                    resolver: Policy::from_rewards(
                        vec![(SKIP, 0.0), (ENTER, Reward::MAX / 2.0)],
                        game.active_player().other(),
                    ),
                    alt: Player::best_value(game.active_player()),
                    prior_probability: 1.0,
                    children: vec![History::new(game.encode())],
                }),
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
        self.subgame_root
            .children
            .iter()
            .map(|h| {
                let ResolverGadget { resolver, .. } = h;
                resolver.p_exploit(&ENTER)
            })
            .fold(0.0, f64::max)
    }

    fn drain_root(&mut self) -> Vec<ResolverGadget<G>> {
        // Mem swap out the vec from root
        // let mut j0 = Self::get_j0(&mut self.subgame_root);
        let mut j0 = Vec::new();
        std::mem::swap(&mut j0, &mut self.subgame_root.children);
        j0
    }

    fn drain_resolver(resolver: &mut ResolverGadget<G>) -> Vec<History<G>> {
        let mut j0 = Vec::new();
        std::mem::swap(&mut j0, &mut resolver.children);
        j0
    }

    /// Update "now" in the solver
    /// TODO: figure out the initial construction
    pub fn construct_subgame(&mut self, hist: G::Trace, player: Player) {
        // How does ord work for multiple players?
        type PreResolver<G> = (Probability, Reward, Vec<History<G>>);
        let other = player.other();
        // Find all root histories
        // Filter down to the second cover of the trace -> split by opponent infostate (they are kinda doing it by post action infostate)
        let root_histories = self
            .drain_root()
            .into_iter()
            .flat_map(|mut x| Self::drain_resolver(&mut x).into_iter())
            .collect();

        let covered = Self::k_cover(root_histories, HashSet::from([hist.clone()]), 2);
        let mut positions: HashMap<G::Trace, PreResolver<G>> = covered.
            into_iter()
            .fold(HashMap::new(), |mut map, history| {
                let trace = history.trace();
                let my_prob = history.reach_prob(player);  // TODO: check this
                // TODO: should get the value from the infoset
                let info_expectation = 0.0;

                match map.entry(trace) {
                    Entry::Occupied(mut entry) => {
                        let (prob, _alt, vec) = entry.get_mut();
                        *prob += my_prob;
                        vec.push(history);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert((my_prob, info_expectation, vec![history]));
                    }
                }
                map
            });
        // let mut positions = covered.into_iter().gr
        // let mut positions: HashMap<G::Trace, PreResolver<G>> = HashMap::new(); // Resolve node has V_alt,
                                                                               // Augment with additional matching traces
        let mut data_count = positions.len();
        let mut new_positions = G::sample_position(hist);
        while data_count < MIN_INFO_SIZE.min(positions.len()) {
            if let Some(g) = new_positions.next() {
                let s = History::New {
                    state: Box::new(g.encode()),
                };
                let opp_trace = g.trace(other);
                let alt = g.evaluate();
                let resolver_info = (1.0, alt, vec![s.clone()]);
                positions
                    .entry(opp_trace)
                    .and_modify(|e| {
                        e.2.push(s);
                    })
                    .or_insert(resolver_info);
                data_count += 1;
            } else {
                break;
            }
        }
        // Initialize the Resolver Nodes: alt, chance node with Resolver policy

        // Add Root with opp. policy to choose their infostate
        let root = SubgameRoot::new(positions, player);
        self.subgame_root = root;
    }

    fn k_cover(root_histories: Vec<History<G>>, hist: HashSet<G::Trace>, k: u8) -> Vec<History<G>> {
        // Find all nodes matching this trace
        let (mut covered, rest): (Vec<_>, Vec<_>) = root_histories
            .into_iter()
            .partition(|h| hist.contains(&h.trace()));
        // if k > 1, find all nodes in k-1_cover of all other player traces
        if k > 1 {
            covered.extend(Self::k_cover(
                rest,
                covered.iter().map(|h| h.trace()).collect(),
                k - 1,
            )); // FIXME: should be the other player's trace
        }
        covered
    }

    fn make_utilities_hist(
        h: &mut History<G>,
        optimizing_player: Player,
        reach_prob: Probability,
    ) -> Reward {
        match h {
            History::Terminal { payoff } => *payoff,

            History::New { state } => {
                let mut v = History::Visited {
                    state: state.clone(),
                    reach: HashMap::new(),
                };
                Self::make_utilities_hist(&mut v, optimizing_player, reach_prob)
            }
            History::Visited { .. } => {
                // Expand lazily only when necessary (caller should expand before calling utilities normally)
                0.0
            }
            History::Expanded {
                info,
                player,
                children,
                ..
            } => {
                let mut local = 0.0;
                for (a, child) in children.iter_mut() {
                    let p = info.borrow().policy.p_exploit(a);
                    if *player == optimizing_player || p > 0.0 {
                        let v = Self::make_utilities_hist(child, optimizing_player, reach_prob * p);
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
    
    fn expansion_step() {
        
    }
    
    fn solve_step(&mut self) {
        self.cfr_iterations(Player::P1);
        self.cfr_iterations(Player::P2);

        for resolver in self.subgame_root.children.iter_mut() {
            resolver.resolver.set_expectation(&ResolveActions::ENTER, resolver.info.policy.expectation());
        }

        let p_max: Probability = self.get_pmax();
        let maxmargin = &mut self.subgame_root.policy;
        for (idx, child) in self.subgame_root.children.iter_mut().enumerate() {
            // if let History::Augmented { resolver, prior_probability, .. } = child {
            let p_maxmargin = maxmargin.p_exploit(&idx);
            let resolver = &child.resolver;
            let prior_probability = child.prior_probability;
            let p_resolve = resolver.p_exploit(&ENTER);
            let reach_prob = p_max * (prior_probability) * p_resolve + (1.0-p_max) * p_maxmargin;
            maxmargin.set_expectation(&idx, reach_prob);
        }
        maxmargin.update()  // TODO: planning on making policy updates lazy
    }
    
    fn cfr_iterations(&mut self, optimizing_player: Player) {
        let SubgameRoot { policy: ref mut root_policy, ref mut children } = &mut self.subgame_root;
        // let mut root_value = 0.0;
        for (resolver_idx, resolver_gadget) in children.iter_mut().enumerate() {
            let ResolverGadget { resolver, prior_probability, children: histories, alt, info } = resolver_gadget;
            let p_enter = resolver.p_exploit(&ENTER);
            let mut enter_value = 0.0;
            for (h_idx, history) in histories.iter_mut().enumerate() {
                let history_p = *prior_probability * p_enter * 1.0; // TODO: correct this
                let h_value = Self::make_utilities_hist(history, optimizing_player, history_p);
                info.policy.set_expectation(&h_idx, history_p);
                enter_value += history_p * h_value;
            }
            resolver.set_expectation(&ENTER, enter_value);
            resolver.set_expectation(&SKIP, *alt);
            let resolver_value = (1.0 - p_enter) * *alt + p_enter * enter_value;
            root_policy.set_expectation(&resolver_idx, resolver_value);
            // root_value += resolver_value * *prior_probability;
        }
    }

    pub fn make_move(&mut self, observation: G::Trace, player: Player) -> G::Action {
        println!("Making move: {:?}, {:?}", player, observation);
        self.start_time = SystemTime::now();
        self.active_player = player;

        self.construct_subgame(observation.clone(), player);

        // very lightweight loop: expand each J0 member once, then evaluate gadgets
        while self.start_time.elapsed().unwrap_or(Duration::from_secs(0))
            < Duration::from_secs(SOLVE_TIME_SECS)
        {
            // println!("LOOP!");
            // Split-borrow self so we can mutate two fields without tripping the borrow checker
            // let (subgame_root_mut, info_sets_mut) = {
            //     let Self {
            //         subgame_root,
            //         info_sets,
            //         ..
            //     } = self;
            //     (
            //         subgame_root as &mut SubgameRoot<G>,
            //         info_sets as &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>,
            //     )
            // };

            // Evaluate utilities from root
            self.solve_step();
            // TODO: add the expand step
        }

        // return purified best from chosen expanded node; if missing, fall back to random on any infoset for player
        if let Some(a) = self.choose_action_from_root() {
            return a;
        }

        // Fallback: pick an action from any infoset for the player
        for (_t, rc) in self.info_sets.iter() {
            let info = rc.borrow();
            if info.player == player && !info.policy.actions.is_empty() {
                return info.policy.purified();
            }
        }

        panic!("No action available");
    }
}

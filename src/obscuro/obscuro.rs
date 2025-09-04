use crate::obscuro::history::*;
use crate::obscuro::info::*;
use crate::obscuro::obscuro::ResolveActions::{ENTER, SKIP};
use crate::obscuro::policy::Policy;
use crate::obscuro::utils::*;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::time::{Duration, SystemTime};
use rand::distributions::{Distribution, WeightedIndex};
use rand::thread_rng;

// ---------- Resolve gadget ----------
struct SubgameRoot<G: Game> {
    policy: Policy<usize>,
    children: Vec<ResolverGadget<G>>,
}
impl<G: Game> SubgameRoot<G> {
    /// Create a new subgame root from 2-cover, all the info-states the other player believes we could be in
    pub fn new(
        j0: HashMap<G::Trace, PreResolver<G>>,
        player: Player,
    ) -> Self {
        // Prior for each J node = given probability (already normalized upstream if desired)
        let mut items: Vec<ResolverGadget<G>> = Vec::new();
        for (trace, (_pp, alt, entries)) in j0.into_iter() {
            // Create an augmented gadget that mixes SKIP (alt) vs. ENTER (children)
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
    info: Info<usize, G::Trace>, // Info policy showing the probability distribution of reach odds for each child in this opp. 
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
            policy: Policy::from_rewards(vec![], game.active_player()),
            children: vec![
                (ResolverGadget {
                    info: Info::from_policy(
                        Policy::from_actions(vec![], game.active_player()),
                        Default::default(),
                        game.active_player(),
                    ),
                    resolver: Policy::from_rewards(
                        vec![(SKIP, 0.0), (ENTER, Reward::MAX / 2.0)],
                        game.active_player().other(),
                    ),
                    alt: Player::best_value(game.active_player()),
                    prior_probability: 1.0,
                    children: vec![],
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
    /// How likely is it that the opponent would ever enter this subgame
    fn get_pmax(&mut self) -> Probability {
        self.subgame_root
            .children
            .iter_mut()
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

        let mut data_count = positions.len();
        let mut new_positions = G::sample_position(hist.clone());
        while data_count < MIN_INFO_SIZE {
            if let Some(g) = new_positions.next() {
                let s = History::new(g.clone(), HashMap::new());  // Start with probability 1.0 (relative to its root)
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
        
        println!("Constructed subgame with {} positions", positions.len());
        // print hist
        println!("Hist: {:?}", hist);
        debug_assert!(!positions.is_empty());
        // Initialize the Resolver Nodes: alt, chance node with Resolver policy

        // Add Root with opponent policy to choose their info-state
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

    fn make_utilities(
        h: &mut History<G>,
        optimizing_player: Player,
        reach_prob: HashMap<Player, Probability>,
    ) -> Reward {
        match h {
            History::Terminal { payoff } => *payoff,
            
            History::Visited { reach, payoff, .. } => {
                *reach = reach_prob.clone();
                *payoff
            }
            History::Expanded {
                info,
                player,
                children,
                reach,
                ..
            } => {
                // TODO: update the reach probabilities
                *reach = reach_prob.clone();
                let mut local = 0.0;
                for (a, child) in children.iter_mut() {
                    let action_chance = info.borrow_mut().policy.p_exploit(a);
                    let mut new_reach_prob = reach_prob.clone();
                    new_reach_prob.entry(*player).and_modify(|x| *x *= action_chance).or_insert(action_chance);
                    if *player == optimizing_player || action_chance > 0.0 {
                        let v = Self::make_utilities(child, optimizing_player, new_reach_prob);
                        local += action_chance * v;
                        info.borrow_mut().add_counterfactuals(a.clone(), v);
                    }
                }
                local
            }
        }
    }
    fn choose_action_from_root(&mut self) -> Option<G::Action> {
        let best_idx = self.subgame_root.policy.best_action();
        let best = &mut self.subgame_root.children[best_idx];
        if let Some(History::Expanded {info, .. }) = best.children.iter_mut().find(|h| { matches!(h, History::Expanded { .. }) }) {
            Some(info.borrow_mut().policy.purified())
        } else {
            unreachable!()
        }
    }
    fn expansion_step(&mut self) {
        println!("Expansion step");
        let Self {subgame_root, info_sets, ..} = self;
        let hist1 = Self::sample_history(subgame_root);
        println!("Source1:");
        hist1.print_family();
        Obscuro::expansion_step_inner(Player::P1, hist1, info_sets);
        let Self {subgame_root, info_sets, ..} = self;
        let hist2 = Self::sample_history(subgame_root);
        println!("Source2:");
        hist2.print_family();
        Obscuro::expansion_step_inner(Player::P2, hist2, info_sets);
    }
    // Adjust names/tuple shapes to your actual types.
    fn sample_history(subgame_root: &mut SubgameRoot<G>) -> &mut History<G> {
        // 1) Collect coordinates and weights in a short scope so borrows end before we reborrow mutably.
        let (coords, probs) = {
            let mut coords: Vec<(usize, usize)> = Vec::new();
            let mut probs: Vec<Probability> = Vec::new();

            // Use `iter()` or `iter_mut()` depending on what `net_reach_prob()` needs.
            for (i, aug) in subgame_root.children.iter().enumerate() {
                for (j, h) in aug.children.iter().enumerate() {
                    probs.push(h.net_reach_prob());
                    coords.push((i, j));
                }
            }

            (coords, probs)
        };

        // 2) Sample an index from the weights.
        let dist = WeightedIndex::new(&probs).expect("no options / invalid weights");
        let k = dist.sample(&mut thread_rng());
        let (i, j) = coords[k];

        // 3) Reborrow mutably and return the selected history.
        &mut subgame_root.children[i].children[j]
    }
    fn expansion_step_inner(player: Player, mut here: &mut History<G>, infosets: &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>) {
        while let History::Expanded {info, children, player: here_player, .. } = here {
            let policy: &mut Policy<G::Action> = &mut info.borrow_mut().policy;
            let action = if *here_player==player {
                policy.explore()
            } else {
                policy.exploit()
            };
            policy.add_expansion(&action);
            here = children.iter_mut()
                .find(|(ca, _)| *ca==action)
                .map(|(_, ch)| ch)
                .unwrap();
        }
        match here {
            History::Expanded {..} => unreachable!(),
            History::Terminal {..} => (),
            History::Visited {..} => here.expand(infosets),
        }
    }
    
    fn solve_step(&mut self) {
        // println!("Solve step");
        self.cfr_iterations(Player::P1);
        self.cfr_iterations(Player::P2);

        for resolver in self.subgame_root.children.iter_mut() {
            resolver.resolver.set_expectation(&ENTER, resolver.info.policy.expectation());
        }

        let p_max: Probability = self.get_pmax();
        let maxmargin = &mut self.subgame_root.policy;
        for (idx, child) in self.subgame_root.children.iter_mut().enumerate() {
            // if let History::Augmented { resolver, prior_probability, .. } = child {
            let p_maxmargin = maxmargin.p_exploit(&idx);
            let resolver = &mut child.resolver;
            let prior_probability = child.prior_probability;
            let p_resolve = resolver.p_exploit(&ENTER);
            let reach_prob = p_max * (prior_probability) * p_resolve + (1.0-p_max) * p_maxmargin;
            maxmargin.set_expectation(&idx, reach_prob);
        }

    }
    
    fn cfr_iterations(&mut self, optimizing_player: Player) {
        let SubgameRoot { policy: ref mut root_policy, ref mut children } = &mut self.subgame_root;
        // let mut root_value = 0.0;
        let resolver_dist = root_policy.exploit_policy();
        for (resolver_idx, (resolver_gadget, r_prob)) in children.iter_mut().zip(resolver_dist).enumerate() {
            let ResolverGadget { resolver, prior_probability, children: histories, alt, info } = resolver_gadget;
            // TODO: what is prior prob for?
            let p_enter = 1.0;//resolver.p_exploit(&ENTER);
            let mut enter_value = 0.0;
            let distribution = info.policy.exploit_policy();
            for (h_idx, (history, sample_chance)) in histories.iter_mut().zip(distribution.iter()).enumerate() {
                // println!("h_idx: {}, sample_chance: {}", h_idx, sample_chance);
                // println!("Enter Chance: {} (alt={})", p_enter, alt);
                let action_reach = HashMap::from([
                        (Player::Random, *sample_chance),
                        (optimizing_player.other(), r_prob * p_enter),
                    ]);
                let h_value = Self::make_utilities(history, optimizing_player, action_reach);
                info.policy.set_expectation(&h_idx, h_value);
                enter_value += sample_chance * h_value;
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
        while self.start_time.elapsed().unwrap_or(Duration::from_secs(0)) < Duration::from_secs(SOLVE_TIME_SECS) {
            // Evaluate utilities from root
            self.expansion_step();
            for _ in 0..100 {
                self.solve_step();
            }
        }

        // return purified best from chosen expanded node; if missing, fall back to random on any infoset for player
        if let Some(a) = self.choose_action_from_root() {
            return a;
        }

        // Fallback: pick an action from any infoset for the player
        for (_t, rc) in self.info_sets.iter_mut() {
            let mut info = rc.borrow_mut();
            if info.player == player && !info.policy.actions.is_empty() {
                return info.policy.purified();
            }
        }

        panic!("No action available");
    }
}

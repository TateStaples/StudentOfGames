use super::history::*;
use crate::info::*;
use crate::obscuro::ResolveActions::{ENTER, SKIP};
use crate::policy::Policy;
use crate::utils::*;
use rand::distr::weighted::WeightedIndex;
use rand::prelude::Distribution;
use rand::rng;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};

/// The solving engine of the project. Combines CFR, AlphaZero tree growth, & safe resolving
pub struct Obscuro<G: Game> {
    pub expectation: Reward,
    total_updates: usize,
    info_sets: HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>,
    subgame_root: SubgameRoot<G>,
    solver: G::Solver,
    start_time: SystemTime,
}

impl<G: Game> Obscuro<G> {
    /// Develop a strategy and then return the action you have decided
    pub fn make_move(&mut self, observation: G::Trace, player: Player) -> G::Action {
        debug_assert!(!matches!(player, Player::Chance));
        self.study_position(observation.clone(), player);
        // return purified best from the chosen expanded node; if missing, fall back to random on any infoset for player
        // println!("I see {:?}, out of {:?}", observation, self.info_sets.keys());
        self.info_sets[&observation].borrow().policy.purified()
    }

    pub fn inst_policy(&self, observation: G::Trace) -> Policy<G::Action> {
        self.info_sets[&observation].borrow().policy.clone()
    }
    /// Given a observation, update you understanding of game state & strategy
    pub fn study_position(&mut self, observation: G::Trace, player: Player) {
        // println!("Making move: {:?}, {:?}", player, observation);
        self.start_time = SystemTime::now();

        self.construct_subgame(observation.clone(), player);

        while self.start_time.elapsed().unwrap_or(Duration::from_secs(0)) < Duration::from_millis((SOLVE_TIME_SECS*1000.0) as u64) {
            self.expansion_step();
            for _ in 0..10 {
                self.solve_step();
            }
        }
        println!("SIZE: {}", self.size());
        // self.debug();
    }
    // Below is all private functions necesary for growing tree CFR & safe resolving
    // ---------- Algo Setup --------- //
    /// Take in an observation, filter down to the new relevant tree (being careful about safe-replay)
    fn construct_subgame(&mut self, hist: G::Trace, player: Player) {  // TODO: split up construct subgame -> function is too long
        // TODO: reset the game expectation
        debug_assert!(!matches!(player, Player::Chance));
        // Find all root histories

        let mut positions = self.pop_histories(hist.clone(), player);
        let v_star = self.expectation; // Previous search expectation
        Self::populate_histories(&mut positions, hist, player, v_star);

        // Renormalize all the histories to sum to 1.0 (probability of being in this subgame)
        let total_prob = positions.iter().map(|(_, (prob, _, _))| *prob).sum::<Probability>();
        for (_, (_, _, hists)) in positions.iter_mut() {
            for h in hists {
                h.renormalize_reach(total_prob);
            }
        }
        debug_assert!(!positions.is_empty());
        // Initialize the Resolver Nodes: alt, chance node with Resolver policy

        // Add Root with opponent policy to choose their info-state
        let root = SubgameRoot::new(positions, player);
        self.subgame_root = root;
    }
    fn pop_histories(&mut self, hist: G::Trace, player: Player) -> HashMap<G::Trace, PreResolver<G>> {
        // Filter down to the second cover of the trace -> split by opponent infostate (they are kinda doing it by post-action infostate)
        let root_histories = self.drain_root()
            .into_iter()
            .flat_map(|mut x| Self::drain_resolver(&mut x).into_iter())
            .collect();
        // println!("Root histories: {:?}", root_histories);
        let mut covered = Self::k_cover(root_histories, hist.clone(), player, 3);
        let new_possibility = covered.iter().map(|x| x.net_reach_prob()).sum::<Probability>();
        for x in covered.iter_mut() {
            x.renormalize_reach(new_possibility);
        }
        // println!("Covered: {:?}", covered);
        let mut positions: HashMap<G::Trace, PreResolver<G>> = covered.
            into_iter()
            .fold(HashMap::new(), |mut map, history| {
                let trace = history.trace();
                let my_prob = history.net_reach_prob();
                
                // Compute u(x,y|J) - the expected value under current strategies
                let info_expectation = match &history {
                    History::Expanded {..} => self.info_sets[&trace].borrow().policy.expectation(),
                    History::Terminal {payoff,..} | History::Visited {payoff,..} => *payoff,
                };
                
                // Compute gift value ĝ(J) = sum of positive counterfactual advantages along path
                let gift_value = Self::compute_gift_value(&history, player);
                
                // Alternate value: v_alt(J) = u(x,y|J) - ĝ(J)
                let alt_value = info_expectation - gift_value;

                match map.entry(trace) {
                    Entry::Occupied(mut entry) => {
                        let (prob, _alt, vec) = entry.get_mut();
                        *prob += my_prob;
                        vec.push(history);
                    }
                    Entry::Vacant(entry) => {
                        entry.insert((my_prob, alt_value, vec![history]));
                    }
                }
                map
            });
        positions
    }
    
    /// Compute gift value ĝ(J) = Σ [u_cf(x,y; J'a') - u_cf(x,y; J')]_+
    /// This represents the advantage gained from opponent mistakes leading to J
    fn compute_gift_value(history: &History<G>, player: Player) -> Reward {
        match history {
            History::Terminal { .. } | History::Visited { .. } => 0.0,
            History::Expanded { info, children, player: hist_player, .. } => {
                if *hist_player == player.other() {
                    // At opponent nodes, sum positive counterfactual advantages
                    let policy = info.borrow();
                    let current_value = policy.policy.expectation();
                    
                    let mut gift = 0.0;
                    for (_action, child) in children.iter() {
                        let child_value = child.payoff();
                        let advantage = (child_value - current_value).max(0.0);
                        gift += advantage;
                        
                        // Recursively add gifts from descendants
                        gift += Self::compute_gift_value(child, player);
                    }
                    gift
                } else {
                    // At our nodes or chance nodes, just sum recursively
                    let mut gift = 0.0;
                    for (_, child) in children.iter() {
                        gift += Self::compute_gift_value(child, player);
                    }
                    gift
                }
            }
        }
    }
    fn populate_histories(positions: &mut HashMap<G::Trace, PreResolver<G>>, hist: G::Trace, player: Player, v_star: Reward) {
        let mut data_count = positions.len();
        let mut new_positions = G::sample_position(hist.clone());
        let other = player.other();

        while data_count < MIN_INFO_SIZE {
            if let Some(g) = new_positions.next() {
                let game_hash = g.identifier();
                if positions.iter()
                    .flat_map(|(_, (_, _, v))|
                        v.iter().map(|x|x.identifier()))
                    .any(|x| x == game_hash) {
                    continue;
                }
                println!("Constructing new position: {:?}", g);
                let s = History::new(g.clone(), HashMap::new());  // Start with probability 1.0 (relative to its root)
                let opp_trace = g.trace(other);
                
                // For newly-sampled states, alternate value is min(stockfish_eval, v*)
                // where v* is the expected value from previous search
                let stockfish_eval = g.evaluate();
                let alt = stockfish_eval.min(v_star);
                
                positions
                    .entry(opp_trace)
                    .or_insert( (1.0, alt, vec![]))
                    .2.push(s);
                data_count += 1;
            } else {
                break;
            }
        }
    }
    /// Find all histories still believed to be possible up to order-k. Solved by iterating to fixed point
    /// k=1 => I believe we could be here
    /// k=2 => I know They believe we could be here
    /// k=3 => They think I believe believe
    fn k_cover(mut root_histories: Vec<History<G>>, hist: G::Trace, mut player: Player, k: u8) -> Vec<History<G>> {
        // k=1, what do I think it might be, k=2, what might they think it might be, k=3: what might they think I might think
        // Can get k=\infty by iterating to a fixed point
        debug_assert!(!matches!(player, Player::Chance));
        if root_histories.is_empty() {return vec![]}
        let mut search_for = HashSet::from([hist.clone()]);
        let mut all_found = vec![];
        for _ in 0..k {
            let mut next_roots = vec![];
            let mut next_search_for = HashSet::new();
            for root in root_histories.drain(0..root_histories.len()) {
                let (next_root, new_nodes, new_search_for) = Self::k_cover_rec(root, &search_for, player);
                if let Some(next_root) = next_root {
                    next_roots.push(next_root);
                }
                next_search_for.extend(new_search_for);
                all_found.extend(new_nodes);
            }
            std::mem::swap(&mut root_histories, &mut next_roots);
            std::mem::swap(&mut search_for, &mut next_search_for);
            player = player.other();
        }
        all_found
    }

    /// Private recursive call for single fixed point search
    /// Looks to see which nodes match a set of Trace
    fn k_cover_rec(mut root: History<G>, hist: &HashSet<G::Trace>, player: Player) -> (Option<History<G>>, Vec<History<G>>, HashSet<G::Trace>) {
        // println!("k_cover_rec: {:?} looking for {:?} ({:?})", root, hist, player);
        if matches!(root, History::Terminal { .. }) {
            return (Some(root), vec![], HashSet::new())
        }
        let my_trace = root.players_view(player);
        let comparisons: Vec<std::cmp::Ordering> = hist.iter()
            .filter_map(|x| {
                // println!("Comparing: t->{:?} & h->{:?} = {:?}", x.clone(), my_trace.clone(), my_trace.partial_cmp(x));
                my_trace.partial_cmp(x)
            })
            .collect();
        // println!("Comparisons: {:?}, root({:?}): {:?}, my_trace: {:?}, target: {:?}", comparisons, player,root, my_trace, hist);
        debug_assert!(!comparisons.contains(&std::cmp::Ordering::Greater), "{:?} > {:?}", my_trace, hist);
        if comparisons.contains(&std::cmp::Ordering::Equal) {
            // return this as a vec and traces of all of my children
            let other_view = root.players_view(root.player().other());
            (None, vec![root], HashSet::from([other_view]))
        } else if !comparisons.is_empty() && matches!(root, History::Expanded { .. }) {
            // Pull children out safely (without moving `root`)
            let children_vec = if let History::Expanded { children, .. } = &mut root {
                // println!("Exploring their children: {:?}", children);
                std::mem::take(children) // leaves `children` as empty vec
            } else {
                unreachable!()
            };

            // Process and build the replacement children + accumulators
            let (new_children, hits, views) = children_vec
                .into_iter()
                .fold((Vec::new(), Vec::new(), HashSet::new()), |(mut cs, mut hs, mut vs),
                                                                 (action, child)| {
                    let (back, found, new_views) = Self::k_cover_rec(child, hist, player);
                    if let Some(back) = back {
                        cs.push((action, back));
                    }
                    hs.extend(found);
                    vs.extend(new_views);
                    (cs, hs, vs)
                });

            // Write the new children back into `root`
            if let History::Expanded { children, .. } = &mut root {
                *children = new_children;
            }

            (Some(root), hits, views)
        } else {
            (Some(root), vec![], HashSet::new())
        }
    }
    // ---------- Tree Growth --------- //
    fn expansion_step(&mut self) {
        // println!("Expansion step");
        let Self {subgame_root, info_sets, ..} = self;
        let hist1 = Self::sample_history(subgame_root);
        Obscuro::expansion_step_inner(Player::P1, hist1, info_sets);
        let Self {subgame_root, info_sets, ..} = self;
        let hist2 = Self::sample_history(subgame_root);
        Obscuro::expansion_step_inner(Player::P2, hist2, info_sets);
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
    // ---------- CFR+ --------- //
    fn solve_step(&mut self) {
        self.cfr_iterations(Player::P1);
        self.cfr_iterations(Player::P2);

        let p_max: Probability = self.get_pmax();
        let maxmargin = &mut self.subgame_root.maxmargin;
        // println!("root children: {:?}", self.subgame_root.children.iter().map(|x|&x.info).collect::<Vec<_>>());
        for (idx, child) in self.subgame_root.children.iter_mut().enumerate() {
            // if let History::Augmented { resolver, prior_probability, .. } = child {
            let p_maxmargin = maxmargin.p_exploit(&idx);
            let resolver = &mut child.resolver;
            let prior_probability = child.prior_probability;
            let p_resolve = resolver.p_exploit(&ENTER);
            let reach_prob = p_max * (prior_probability) * p_resolve + (1.0-p_max) * p_maxmargin;
            maxmargin.add_counterfactual(&idx, reach_prob, 1.0);
        }

    }
    
    fn cfr_iterations(&mut self, optimizing_player: Player) {
        self.total_updates += 1;
        let SubgameRoot { maxmargin: ref mut root_policy, ref mut children } = &mut self.subgame_root;
        let mut root_value = 0.0;
        let resolver_dist = root_policy.inst_policy();
        for (resolver_idx, (resolver_gadget, r_prob)) in children.iter_mut().zip(resolver_dist).enumerate() {
            let ResolverGadget { resolver, alt, children: histories, info, prior_probability } = resolver_gadget;
            let p_enter = resolver.p_exploit(&ENTER);
            let mut enter_value = 0.0;
            let distribution = info.policy.inst_policy();
            for (_, (history, sample_chance)) in histories.iter_mut().zip(distribution.iter()).enumerate() {
                // println!("h_idx: {}, sample_chance: {}", h_idx, sample_chance);
                // println!("Enter Chance: {} (alt={})", p_enter, alt);
                let action_reach = HashMap::from([
                        (Player::Chance, *sample_chance),
                        (optimizing_player.other(), r_prob * p_enter),
                    ]);
                let h_value = Self::make_utilities(history, optimizing_player, action_reach);
                Self::apply_updates(history, self.total_updates);
                enter_value += sample_chance * h_value;
            }
            resolver.add_counterfactual(&ENTER, enter_value, r_prob);
            resolver.add_counterfactual(&SKIP, *alt, r_prob);
            resolver.update(self.total_updates);
            let resolver_value = (1.0 - p_enter) * *alt + p_enter * enter_value;
            root_policy.add_counterfactual(&resolver_idx, resolver_value, 1.0);
            root_value += resolver_value * *prior_probability;
        }
        root_policy.update(self.total_updates);
    }

    /// Calculate the weighted expectation of all actions on history (and children) & update policy
    fn make_utilities(h: &mut History<G>, optimizing_player: Player, reach_prob: HashMap<Player, Probability>) -> Reward {
        match h {
            History::Terminal { payoff, .. } => *payoff,
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
                let net_reach_prob: Probability = reach_prob.get(&Player::Chance).unwrap_or(&1.0) * reach_prob.get(&player.other()).unwrap_or(&1.0);
                *reach = reach_prob.clone();
                let mut local = 0.0;
                let policy = &mut info.borrow_mut().policy;
                let distribution = policy.inst_policy();
                for ((a, child), action_chance) in children.iter_mut().zip(distribution.iter()) {
                    let mut new_reach_prob = reach_prob.clone();
                    new_reach_prob.entry(*player).and_modify(|x| *x *= action_chance).or_insert(*action_chance);
                    if *player == optimizing_player || action_chance > &0.0 {
                        let v = Self::make_utilities(child, optimizing_player, new_reach_prob);
                        local += action_chance * v;
                        // println!("Adding Counterfactual {:?}: v={:.2}, cfvs={:.2}, update={}, rp={}, ac={:.2},action={:?}, trace={:?}", player, v, cfvs, total_updates, net_reach_prob, action_chance, a.clone(), trace);
                        policy.add_counterfactual(a, v, net_reach_prob);
                    }
                }
                local
            }
        }
    }
    
    /// Recursively use the counterfactuals learned in CFR to update all the policies
    fn apply_updates(h: &mut History<G>, total_updates: usize) {
        match h {
            History::Terminal {..} => (),
            History::Visited {..} => (),
            History::Expanded {children, info, .. } => {
                for (_, child) in children.iter_mut() {
                    Self::apply_updates(child, total_updates);
                }
                info.borrow_mut().policy.update(total_updates);
            }
        }
    }
    // ---------- Util Functions --------- //
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
            debug_assert!(!probs.is_empty()&&probs.iter().sum::<Probability>() > 1e-12);
            (coords, probs)
        };

        // 2) Sample an index from the weights.
        let dist = WeightedIndex::new(&probs).expect("no options / invalid weights");
        let k = dist.sample(&mut rng());
        let (i, j) = coords[k];

        // 3) Reborrow mutably and return the selected history.
        &mut subgame_root.children[i].children[j]
    }
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

    /// Get ownership all the resolver gadgets (leaving root empty)
    fn drain_root(&mut self) -> Vec<ResolverGadget<G>> {
        // Mem swap out the vec from root
        // let mut j0 = Self::get_j0(&mut self.subgame_root);
        let mut j0 = Vec::new();
        std::mem::swap(&mut j0, &mut self.subgame_root.children);
        j0
    }
    /// Get ownership of all possible history nodes in a opponent resolver gadget
    fn drain_resolver(resolver: &mut ResolverGadget<G>) -> Vec<History<G>> {  // TODO: maybe just take ownership of the resolver
        let mut j0 = Vec::new();
        std::mem::swap(&mut j0, &mut resolver.children);
        j0
    }

    fn size(&self) -> usize {
        self.subgame_root.children.iter().map(|x| {
            x.children.iter().map(|h| h.size()).sum::<usize>()
        }).sum::<usize>()
    }

    pub fn debug(&mut self) {
        println!("SIZE: {}, steps: {}", self.size(), self.total_updates);
        // self.subgame_root.children[0].children[0].print_family();
        Self::make_utilities(&mut self.subgame_root.children[0].children[0], Player::P1, HashMap::new());
        for (trace, info) in self.info_sets.iter_mut() {
            if info.borrow().player != Player::Chance {
                println!("{:?} -> {:?}", trace, info.borrow().policy);
            }
        }
    }
}
impl<G: Game> Default for Obscuro<G> {
    fn default() -> Self {
        let game = G::new();
        let root = SubgameRoot {
            maxmargin: Policy::from_rewards(vec![], game.active_player()),
            children: vec![
                (ResolverGadget {
                    info: Info::from_policy(
                        Policy::from_rewards(vec![], game.active_player()),
                        Default::default(),
                        game.active_player(),
                    ),
                    resolver: Policy::from_rewards(
                        vec![(SKIP, 0.0), (ENTER, 1.0)],
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
            total_updates: 10,
            info_sets: HashMap::new(),
            subgame_root: root,
            start_time: SystemTime::now(),
        }
    }
}

// ---------- Safe Resolving Types ----------
/// Root of the game tree. Holds information necesary to perform safe-resolving on top of the GT-CFR
struct SubgameRoot<G: Game> {
    maxmargin: Policy<usize>,
    children: Vec<ResolverGadget<G>>,
}
impl<G: Game> SubgameRoot<G> {
    /// Create a new subgame root from 2-cover, all the info-states the other player believes we could be in
    pub fn new(
        j0: HashMap<G::Trace, PreResolver<G>>,
        player: Player,
    ) -> Self {
        // Compute prior probabilities α(J) = 1/2 * (1/m + y(J)/Σy(J'))
        // where m is the number of opponent infosets and y(J) is opponent's strategy probability
        let m = j0.len() as Reward;
        let mut y_values: Vec<(G::Trace, Reward)> = Vec::new();
        
        // Collect y(J) values (prior probabilities from belief distribution)
        for (trace, (prior_prob, _, _)) in j0.iter() {
            y_values.push((trace.clone(), *prior_prob));
        }
        
        let sum_y: Reward = y_values.iter().map(|(_, y)| *y).sum();
        
        // Prior for each J node using the paper's formula
        let mut items: Vec<ResolverGadget<G>> = Vec::new();
        for (trace, (prior_prob, alt, entries)) in j0.into_iter() {
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
                trace.clone(),
                Player::Chance,
            );
            let resolver = Policy::from_rewards(vec![(SKIP, alt), (ENTER, 0.0)], player.other());
            
            // Compute α(J) = 1/2 * (1/m + y(J)/Σy(J'))
            let uniform_component = 1.0 / m;
            let belief_component = if sum_y > 0.0 { prior_prob / sum_y } else { 0.0 };
            let alpha_j = 0.5 * (uniform_component + belief_component);

            let augmented = ResolverGadget {
                info,
                resolver,
                alt,
                prior_probability: alpha_j,
                children: entries,
            };
            items.push(augmented);
        }
        
        // Create root policy using the computed prior probabilities
        let root_policy = Policy::from_rewards(
            items.iter().enumerate().map(|(i, r)| (i, r.prior_probability)).collect(),
            player
        );

        debug_assert!(root_policy.actions.len() == items.len());
        SubgameRoot {
            maxmargin: root_policy,
            children: items,
        }
    }

    pub fn print_family(&self) {
        println!("Subgame Root:");
        for (idx, child) in self.children.iter().enumerate() {
            print!("    Resolver Gadget {}:", idx);
            child.print_family();
        }
    }
}
/// Safe-Resolving Gadget to determine whether the opponent would enter this subgame.
struct ResolverGadget<G: Game> {
    info: Info<usize, G::Trace>, // Info policy showing the probability distribution of reach odds for each child in this opp.
    resolver: Policy<ResolveActions>,
    alt: Reward,
    prior_probability: Probability,
    children: Vec<History<G>>,
}
/// Temporary datatype to hold information necesary to build the ResolverGadget
type PreResolver<G> = (Probability, Reward, Vec<History<G>>);
impl<G: Game> ResolverGadget<G> {
    pub fn print_family(&self) {
        println!("RG({:?})", self.info.trace);
        for child in &self.children {
            child.print_family_rec(2, 5);
        }
    }
}
/// For exploitablity gaurentees, opp. given the option to either enter or take 
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ResolveActions {   // TODO: maybe add the alt value into the SKIP action
    SKIP,
    ENTER,
}

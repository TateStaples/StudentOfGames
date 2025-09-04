use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::IteratorRandom;
use rand::thread_rng;
use std::cell::RefCell;
// Imports
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

// Constants
const SOLVE_TIME: u64 = 5;
const MIN_INFO_SIZE: usize = 200;
const MAX_SUPPORT: usize = 3;
const EXPANDERS: usize = 1;
const PLAYER_COUNT: usize = 2;

// Data types
pub type Reward = f64;
pub type Probability = f64;
pub trait ActionI = Clone + Eq + Hash + Copy + PartialEq + Eq;
pub trait StateI = Clone + Eq + Hash + Copy;
pub trait ObservableI = Clone + Hash + Eq;
pub trait TraceI = Clone + Hash + Eq + PartialOrd + Default;
#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
enum Player {
    P1, P2, CHANCE, TERMINAL
}
impl Player {
    fn other(&self) -> Player {
        match self {
            Player::P1 => Player::P2,
            Player::P2 => Player::P1,
            _ => unreachable!("You should not be here")
        }
    }

}
#[derive(Clone, PartialEq, Eq, Hash, Copy)]
enum ResolveActions {
    ENTER, SKIP
}
/// Trait representing a specific game domain (imperfect-information, two-player, zero-sum).
pub trait Game where Self: Sized {
    type State: StateI;
    type Action: ActionI;
    type Observation: ObservableI;
    type Trace: TraceI;  // Some(<) => subgame, None => different branch

    fn encode(&self) -> Self::State;
    fn decode(state: &Self::State) -> Self;
    fn trace(&self, player: Player) -> Self::Trace;
    fn perspective(&self, trace: Self::Trace) -> Player;
    fn active_player(&self) -> Player;
    fn available_actions(&self) -> Vec<Self::Action>;
    fn observation(&self, player: Player) -> Self::Observation;
    fn play(&self, action: &Self::Action) -> Self;
    fn is_over(&self) -> bool;
    fn evaluate(&self) -> Reward;

    fn sample_positions(observation_history: &Self::Trace) -> Vec<Self>;
}
struct Policy<A>{
    multiplier: i8,  // -1, 0, +1
    actions: Vec<A>,
    expectations: Vec<Reward>,
    expansions: Vec<usize>,
    acc_regrets: Vec<Reward>,
    stable: Vec<bool>,
    update: usize,
}
impl<A: ActionI> Policy<A> {  // Mostly done
    const C: f32 = 1.0;
    type ActionPolicy = Vec<Probability>;
    fn from_rewards(rewards: Vec<(A, Reward)>, player: Player) -> Self {
        let multiplier = match player {
            Player::P1 => 1,
            Player::P2 => -1,
            Player::CHANCE | Player::TERMINAL  => 0
        };
        let (actions, rewards): (Vec<A>, Vec<Reward>) = rewards.into_iter().unzip();
        Policy {
            multiplier,
            expectations: vec![0.0; actions.len()],
            expansions: vec![0; actions.len()],
            acc_regrets: rewards,
            stable: vec![true; actions.len()],
            update: 0,
            actions,
        }
    }
    // ----- Helper Methods ----- //
    fn avg_variance(&self) -> Reward {1.0}  // FIXME
    fn quality(&self, action_idx: usize) -> Reward {
        self.expectations[action_idx]
            * self.multiplier as Reward
            + self.avg_variance()
            * Reward::sqrt(self.expansions.iter().map(|&x|x as Reward).sum())
            /(1.0 + self.expansions[action_idx] as Reward)
    }
    fn puct(&self) -> Self::ActionPolicy {
        let idx_max = (0..self.actions.len())
            .max_by(|i1, i2| f64::total_cmp(&self.quality(*i1), &self.quality(*i2)))
            .unwrap();
        let mut res = vec![0.0; self.actions.len()];
        res[idx_max] = 1.0;
        res
    }
    fn max(&self) -> Self::ActionPolicy {
        let exploit = self.exploit_policy();
        let support = exploit.iter().filter(|&x| x > &0.0).count() as Probability;
        exploit.iter().map(|p| if p>&0.0 {1.0/support} else {0.0}).collect()
    }
    fn exploration_policy(&self) -> Self::ActionPolicy {
        let puct = self.puct();
        let max = self.max();
        puct.iter().zip(max).map(|(p1, p2)| {
            (p1 + p2)/2.0
        }).collect()
    }
    fn sample_policy(&self, p: &Self::ActionPolicy) -> A {
        let net_prob: Probability = p.iter().sum();
        let mut rng = thread_rng();
        if net_prob == 0.0 {
            // Take random sample of p.keys()
            self.actions.iter().choose(&mut rng).unwrap().clone()
        } else {
            let weights: Vec<Probability> = p.iter().map(|prob| *prob/net_prob).collect();
            let dist = WeightedIndex::new(weights).unwrap();
            let index = dist.sample(&mut rng);
            self.actions[index].clone()
        }
    }
    fn exploit_policy(&self) -> Self::ActionPolicy {
        let net_regret: Reward = self.acc_regrets.iter().sum();
        self.acc_regrets.iter().map(|p| p/net_regret).collect()
    }
    // ----- Interface ----- //
    pub fn update(&mut self) {todo!()}
    pub fn set_expectation(&mut self, a: &A, v: Reward) {
        self.expectations[self.actions.iter().position(|n| *n==*a).unwrap()] = v;
    }
    pub fn add_expansion(&mut self, a: A) {
        self.expansions[self.actions.iter().position(|n| *n == a).unwrap()] += 1;
    }
    pub fn expectation(&self) -> Reward {self.expectations.iter().sum::<Reward>()/(self.expectations.len() as Reward)}
    pub fn p_exploit(&self, a: &A) -> Probability {
        let net_regret: Reward = self.acc_regrets.iter().sum();
        let idx = self.actions.iter().position(|n| *n == *a).expect("Item not found");
        self.acc_regrets[idx] / net_regret
    }
    // ----- Policies ----- //
    pub fn explore(&self) -> A {
        self.sample_policy(&self.exploration_policy())
    }
    pub fn exploit(&self) -> A {
        self.sample_policy(&self.exploit_policy())
    }
    pub fn purified(&self) -> A {
        let exploit = self.exploit_policy();
        let best_p = exploit.iter().max_by(|a, b| Probability::total_cmp(a, b)).unwrap();
        let mut orders: Vec<(usize, &Probability)> = exploit.iter().enumerate()
            .filter(|(idx, p)| self.stable[*idx] || *p==best_p).collect();
        orders.sort_by(|(_, p1), (_, p2)| Probability::total_cmp(p1, p2));
        let mut support = vec![0.0; self.actions.len()];
        for i in 0..MAX_SUPPORT {
            let (idx, _) = orders[i];
            support[idx] = self.acc_regrets[idx] as Probability;
        }
        self.sample_policy(&support)
    }
}

enum History<G: Game> { // TODO: move into a enum, terminal don't have children, decision don't have payoff
    Terminal {
        payoff: Reward,
    },
    New {
        state: Box<G::State>,
    },
    Visited{
        state: Box<G::State>,
        reach: HashMap<Player, Probability>,
    },
    Expanded{
        info: InfoPtr<G::Action, G::Trace>,
        reach: HashMap<Player, Probability>,
        children: Vec<(G::Action, History<G>)>,
        player: Player,
    },
    Augmented {
        // TODO: add the resolver here
        info: Info<usize, G::Trace>,
        resolver: Policy<ResolveActions>,
        prior_probability: Probability,
        children: Vec<(usize, History<G>)>,
    },
    SubgameRoot {
        // TODO: maybe add prior probs here
        info: Info<usize, G::Trace>,
        children: Vec<(usize, History<G>)>,  // These should all be augmented
    }
}
impl<G: Game> History<G> {
    pub fn new(state: G::State) -> Self {
        History::New {state: Box::new(state)}
    }
    pub fn subgame_root(j0: HashMap<G::Trace, (Probability, Reward, Vec<Self>)>, player: Player) -> Self {
        let m: usize = j0.values().map(|(_, _, c)| c.len()).sum();
        let net_reach: Probability = j0.values().map(|(_, _, c)| c.iter().map(|h| h.player_reach(player)).sum::<Probability>()).sum();
        Self::SubgameRoot {
            info: Info {
                policy: Policy::from_rewards(j0.values().map(|(p, ..)| *p).enumerate().collect(), player),
                trace: Default::default(),
                player,
                reach: Default::default(),
                visited: false,
            },
            children: j0.into_iter().map(|(t, (pp, alt, children))| {
                let reach_prob: Probability = children.iter().map(|h|h.player_reach(player)).sum();
                Self::Augmented {
                    info: Info {  // player is chance
                        policy: Policy::from_rewards(children.iter().map(|c|{
                            c.player_reach(player) as Reward
                        }).enumerate().collect(), player),  // FIXME: this might be the other player
                        trace: Default::default(),
                        player,
                        reach: Default::default(),
                        visited: false,
                    },
                    resolver:  Policy::from_rewards(vec![
                        (ResolveActions::SKIP, alt), (ResolveActions::ENTER, 0.0)
                    ], player.other()),
                    prior_probability: (reach_prob/net_reach + 1.0/(m as Probability))/2.0,
                    children: children.into_iter().enumerate().collect(),
                }
            }).enumerate().collect()
        }
    }
    pub fn visit(&mut self) {
        let state = match std::mem::replace(self, History::Terminal { payoff: Default::default() }) {
            History::New { state } => state,
            _ => panic!("Expected History::New"),
        };
        *self = History::Visited { state };
    }
    fn payoff(&self) -> Reward {
        match self {
            History::New{state, ..} | History::Visited {state, ..} => { G::decode(state).evaluate() },
            History::Terminal {payoff} => *payoff,
            History::Expanded { .. }  => unreachable!("You should not be here"),
            History::SubgameRoot {info, ..} | History::Augmented {info, ..} => info.expectation(),
        }
    }
    fn trace(&self) -> G::Trace {
        match self {
            History::Expanded { info, .. } => info.borrow().trace.clone(),
            History::New {..} | History::Visited { .. } => unreachable!("You should not be here"),
            History::Terminal {..} => unreachable!("You should not be here"),
            History::SubgameRoot {info, ..} | History::Augmented {info, ..} => info.trace.clone(),
        }
    }
    fn player(&self) -> Player { 
        match self {
            History::Terminal { .. } => { unreachable!("You should not be here") },
            History::New { state, .. } => { G::decode(state).active_player() }
            History::Visited { state, .. } => { G::decode(state).active_player()}
            History::Expanded { info, .. } => {info.borrow().player}
            History::Augmented { info, .. }
            | History::SubgameRoot { info, .. } => {info.player}
        }
    }
    fn expand(&mut self, infosets: &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>) {
        match self {
            History::Visited {state, reach, ..} => {
                let game= G::decode(state);
                let player = self.player();
                let mut children = Vec::new();
                for a in game.available_actions() {
                    let new_game = game.play(&a);
                    let child: History<G> = History::new(new_game.encode());
                    children.push((a, child))
                }
                let infostate = self.trace();
                if let Some(rc) = infosets.get(&infostate) {
                    *self = History::Expanded {
                        info: rc.clone(),
                        children,
                        player
                    }
                } else {
                    let info = Info::from_first_history(self);
                    let rc = Rc::new(RefCell::new(info));
                    infosets.insert(infostate, rc.clone());
                    *self = History::Expanded {
                        info: rc,
                        children,
                        player
                    }
                }
            },
            _ => unreachable!("You should not be here")
        }
    }
    fn get_reach(&self) -> &HashMap<Player, Probability> {
        match self {
            History::New{..} | History::Terminal { .. } => unreachable!("You should not be here"),
            History::Expanded {reach, ..} | History::Visited {reach, ..} => reach,
            History::Augmented {prior_probability, .. } => unimplemented!(),
            History::SubgameRoot {..} => unimplemented!()  // maybe just 1
        }
    }
    pub fn player_reach(&self, player: Player) -> Probability {
        *self.get_reach().get(&player).unwrap()
    }
    fn other_players_reach(&self, player: Player) -> Probability {
        self.get_reach().iter().fold(0.0, |x, (p, prob)| {
            if p != &player {x * prob} else {x}
        })
    }
    fn reach_prop(&self) -> Probability {
        self.get_reach().iter().fold(0.0, |x, (_, prob)| {
            x * prob
        })
    }
}
struct Info<A, T> {
    pub policy: Policy<A>,
    trace: T,
    player: Player,
    reach: HashMap<Player, Probability>,
    visited: bool
}
type InfoPtr<A, T> = Rc<RefCell<Info<A, T>>>;

impl<A: ActionI, T: TraceI> Info<A, T> {
    fn from_first_history<G>(h: &mut History<G>) -> Info<G::Action, G::Trace>
    where
        G: Game<Action=A, Trace=T>,
    {
        let available_actions: Vec<G::Action> = h.available_actions();
        Info {
            policy: Policy {
                multiplier: match h.player() {
                    Player::P1 => 1, Player::P2 => -1, _ => 0,
                },
                actions: available_actions,
                expectations: vec![0.0; available_actions.len()],
                expansions: vec![0; available_actions.len()],
                acc_regrets: vec![0.0; available_actions.len()],  // TODO: replace with the other details from Python
                stable: vec![true; available_actions.len()],
                update: 0,
            },
            trace:  h.trace(),
            player: h.player(),
            reach: Default::default(),
            visited: false,
        }
    }

    /// Add one more History‐step from the *same* game G.
    fn add_history<G>(&mut self, h: &mut History<G>)
    where
        G: Game<Action = A, Trace = T>,
    {
        todo!()
    }

    fn gift(&self) -> Reward {
        // agg = 0
        // for child in self.children():
        //     agg += child.gift()
        // agg += max(0.0, child.expectation()-self.expectation())  # this is rediculously inefficient
        // return agg
        0.0 // FIXME
    }
    // fn children(&self) -> Vec<&Self> {todo!()}
    // used for gift (idk how to fix this), filter, subgame sorting (keep J0 policies not as info), cfr updating
    fn expectation(&self) -> Reward {
        self.policy.expectation()
    }
    fn action_idx(&self, action: A) -> usize { 
        self.policy.actions.iter().position(|a| *a == action).unwrap()
    }
    pub fn add_counterfactuals(&mut self, action: A, cfvs: Reward) {
        let idx = self.action_idx(action);
        self.policy.expectations[idx] = cfvs;
    }
    fn update(&mut self) {
        todo!("I want to do some stuff with subscriber count here")
    }
    // fn history(&self) -> Vec<G::Observation> {todo!()}
    // fn player_reach(&self, player: Player) -> Probability {todo!()}
    // fn other_players_reach(&self, player: Player) -> Probability {todo!()}
    // fn reach(&self) -> Probability {todo!()}
}
pub struct Obscuro<G: Game> {
    expectation: Reward,  // TODO: this is never updated
    info_sets: HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>,
    subgame_root: Option<History<G>>,  // Is this the way you want to store this
    start_time: SystemTime,
    // resolvers: HashMap<G::Trace, Policy<ResolveActions>>,
    // prior_probs: HashMap<G::Trace, Probability>,
    active_player: Player,
}
impl<G: Game> Obscuro<G> {
    fn get_j0(&mut self) -> Vec<&mut History<G>> { 
        if let Some(History::SubgameRoot {children, ..}) = &mut self.subgame_root {
            children.iter_mut().map(|(_, p)| p).collect()
        } else {unreachable!()}
    }
    fn get_pmax(&self) -> Probability {
        if let Some(History::SubgameRoot {children, ..}) = &self.subgame_root {
            children.iter().map(|(_, a)| {
                if let History::Augmented {resolver, ..} = a {
                    resolver.p_exploit(&ResolveActions::ENTER)
                } else { unreachable!() }
            }).max_by(|a, b| Probability::total_cmp(a, b)).unwrap_or(0.0)
        } else { unreachable!() }
    }
    fn check_info_life(&mut self) {
        self.info_sets.retain(|_, rc| { Rc::strong_count(rc) <= 1 });
    }
    fn solve_step(&mut self) {
        self.cfr_iterations(Player::P1);
        self.cfr_iterations(Player::P2);

        for J in self.get_j0() {
            if let History::Augmented {resolver, info, ..} = J {
                resolver.set_expectation(&ResolveActions::ENTER, info.expectation());
                resolver.update();
            }
        }

        let p_max: Probability = self.get_pmax();
        if let Some(History::SubgameRoot {info, children}) = &mut self.subgame_root {
            let maxmargin = &mut info.policy;
            for (idx, child) in children {
                if let History::Augmented { resolver, prior_probability, .. } = child {
                    let p_maxmargin = maxmargin.p_exploit(idx);
                    let p_resolve = resolver.p_exploit(&ResolveActions::ENTER);
                    let reach_prob = p_max * (*prior_probability) * p_resolve + (1.0-p_max) * p_maxmargin;
                    maxmargin.set_expectation(idx, reach_prob);
                } else { unreachable!() }
            }
            maxmargin.update()
        } else { unreachable!() }
    }
    fn sample_history(subgame_root: &mut History<G>) -> &mut History<G> {
        if let History::SubgameRoot {children, ..} = subgame_root {
            let (options, probs) = children.iter_mut().map(|(_, c)| {
                if let History::Augmented {children, ..} = c {
                    children.iter_mut().map(|(_, h)| {
                        (h, h.reach_prop())
                    }).collect::<Vec<_>>()
                } else { unreachable!()}
            }).flatten().unzip();//.collect::<Vec<_>>();
            let dist = WeightedIndex::new(probs).unwrap();
            let index = dist.sample(&mut thread_rng());
            options[index]
        } else { unreachable!() }
    }
    fn policy(&self) -> &Policy<G::Action> {
        todo!()
    }
    fn cfr_iterations(&mut self, player: Player) {
        Self::make_utilities(&mut self.subgame_root.as_mut().unwrap(), player, 1.0);

        if player == self.active_player {
            for J in self.get_j0() {

                // TODO subgame_root.policy.expectations[J] += resolvers[J].expectations[ResolveActions.ENTER] * J.policy.multiplier
            }
        }

    }
    fn make_utilities(h: &mut History<G>, optimizing_player: Player, reach_prob: Probability) -> Reward {
        match h {
            History::New { .. } => {
                h.visit();
                Self::make_utilities(h, optimizing_player, reach_prob)
            },
            History::Visited {..} | History::Terminal { .. } => h.payoff(),
            History::Expanded {info, player: local_player, children} => {
                let mut local_counterfactual_values = 0.0;
                for (action, child) in children {
                    let action_prob: Probability = info.borrow().policy.p_exploit(action);
                    if *local_player == optimizing_player || action_prob > 0.0 {
                        let child_cfvs: Reward = action_prob * Self::make_utilities(child, optimizing_player, reach_prob*action_prob);
                        local_counterfactual_values += child_cfvs;
                        info.borrow_mut().add_counterfactuals(action.clone(), child_cfvs);
                    }
                }
                info.borrow_mut().update();  // should only call polciy
                local_counterfactual_values
            }
            History::Augmented { .. } | History::SubgameRoot {..} => {
                todo!()
            }
        }
    }
    fn solver_thread(&mut self) {
        while self.start_time.elapsed().unwrap() < Duration::from_secs(SOLVE_TIME) {
            self.solve_step()
        }
    }
    fn expander_thread(&mut self) {
        while self.start_time.elapsed().unwrap() < Duration::from_secs(SOLVE_TIME) {
            let Self {subgame_root, info_sets, ..} = self;
            let hist1 = Self::sample_history(subgame_root.as_mut().unwrap());
            Obscuro::expansion_step(Player::P1, hist1, info_sets);
            let Self {subgame_root, info_sets, ..} = self;
            let hist2 = Self::sample_history(subgame_root.as_mut().unwrap());
            Obscuro::expansion_step(Player::P2, hist2, info_sets);
        }
    }
    fn expansion_step(player: Player, mut here: &mut History<G>, infosets: &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>) {
        while let History::Expanded {info, children, player: here_player} = here {
            let policy: &mut Policy<G::Action> = &mut info.borrow_mut().policy;
            let action = if *here_player==player {
                policy.explore()
            } else {
                policy.exploit()
            };
            policy.add_expansion(action.clone());
            here = children.iter_mut()
                .find(|(ca, _)| *ca==action)
                .map(|(_, ch)| ch)
                .unwrap();
        }
        match here {
            History::Expanded {..} => unreachable!(),
            History::Terminal {..} | History::New {..} => return,
            History::Visited {..} => here.expand(infosets),
            History::Augmented {..} | History::SubgameRoot {..} => unreachable!(),
        }
    }
    fn no_threading_loop(&mut self) {
        let deadline = Duration::from_secs(SOLVE_TIME);

        while self.start_time.elapsed().unwrap() < deadline {
            self.solve_step();
            let Self {subgame_root, info_sets, ..} = self;
            let hist1 = Self::sample_history(subgame_root.as_mut().unwrap());
            Obscuro::expansion_step(Player::P1, hist1, info_sets);
            let Self {subgame_root, info_sets, ..} = self;
            let hist2 = Self::sample_history(subgame_root.as_mut().unwrap());
            Obscuro::expansion_step(Player::P2, hist2, info_sets);
        }
    }

    fn filter(&mut self, trace: G::Trace) -> Vec<History<G>> {
        todo!()
    }
    fn construct_subgame(&mut self, hist: G::Trace, player: Player) {
        // self.resolvers.clear();
        // self.prior_probs.clear();
        let other_player = player.other();

        let mut positions: Vec<G> = Game::sample_positions(&hist);

        let histories = self.filter(hist);
        let mut data_count = histories.len();
        let mut j0: HashMap<G::Trace, (Probability, Reward, Vec<History<G>>)> = histories.into_iter().map(|h| {
            // get the trace from the perspective of other player
            // let alt = J.expectation() - J.gift();
            todo!()
        }).collect();
        self.subgame_root = None;

        while data_count < MIN_INFO_SIZE.min(positions.len()) {
            let g = positions.pop();
            if let Some(g) = g {
                // TODO: if histories.iter().any(|h| h.trace() == g.trace(Player::TERMINAL)) { continue; }
                let s = History::New { state: Box::new(g.encode()) };
                let payoff = g.evaluate();
                let trace = g.trace(other_player);
                let alt = self.expectation.min(payoff);
                j0.insert(trace, (0.0, alt, vec![s]));
                data_count += 1;
            } else {
                break;
            }
        }
        // TODO: idk if this should be player or other_player
        self.subgame_root = Some(History::subgame_root(j0, player));

    }
    fn make_move(&mut self, hist: G::Trace, player: Player) -> G::Action {
        self.construct_subgame(hist, player);
        self.no_threading_loop();
        let p_max = self.get_pmax();
        if p_max > 0.0 {
            self.policy().purified()
        } else {
            self.policy().exploit()
        }
    }
    fn info_closure(&self) -> Vec<Info<G::Action, G::Trace>> {
        todo!()
    }
}

fn main() {
    // let mut solver = Obscuro {
    //     expectation: 0.0,
    //     info_sets: Default::default(),
    //     subgame_root: None,
    //     start_time: SystemTime::now(),
    //     active_player: Player::P1,
    // };
    // solver.make_move(Default::default(), Player::P1);
}
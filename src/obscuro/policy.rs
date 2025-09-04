use std::cmp::Ordering;
use std::fmt::{write, Debug, Formatter};
use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::IteratorRandom;
use rand::thread_rng;
use crate::obscuro::utils::*;

// ---------- Policy ----------
/// Action Policy 
/// Implements the CFR+ accumulation and action probability calculation
#[derive(Clone)]
pub struct Policy<A: ActionI> {
    multiplier: i8, // +1 for maximizing player (or chance), -1 for minimizing (in zero-sum CFV space)
    pub actions: Vec<A>,
    pub expectations: Vec<Reward>,
    expansions: Vec<usize>,
    pub acc_regrets: Vec<Reward>,
    stable: Vec<bool>,
    num_updates: Reward,
    outdated: bool,
}

impl<A: ActionI> Policy<A> {
    pub fn from_actions(actions: Vec<A>, player: Player) -> Self {
        Self::from_rewards(actions.into_iter().map(|a| (a, 0.0)).collect(), player)
    }

    pub fn from_rewards(items: Vec<(A, Reward)>, player: Player) -> Self {
        let (actions, expectations): (Vec<A>, Vec<Reward>) = items.into_iter().unzip();
        let n = expectations.len();
        Policy {
            multiplier: Self::player_to_multiplier(player),
            actions,
            expectations,
            expansions: vec![0; n],
            acc_regrets: vec![1e-12; n],
            stable: vec![false; n],
            outdated: false,
            num_updates: 1.0,
        }
    }
    
    fn player_to_multiplier(player: Player) -> i8 {
        match player {
            Player::P1 => 1,
            Player::P2 => -1,
            _ => 1,
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

    pub fn exploit_policy(&mut self) -> Vec<Probability> {
        self.update();
        self.lazy_exploit()
    }
    fn lazy_exploit(&self) -> Vec<Probability> {
        if self.actions.is_empty() { return vec![]; }
        let sum: f64 = self.acc_regrets.iter().sum();
        if sum <= 0.0 || !sum.is_finite() {
            // uniform
            let p = 1.0 / (self.actions.len() as f64);
            return vec![p; self.actions.len()];
        }
        self.acc_regrets.iter().map(|r| r / sum).collect()
    }

    fn exploration_policy(&mut self) -> Vec<Probability> {
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

    /// Sample your action policy for exploring more of the space
    pub fn explore(&mut self) -> A { 
        self.update(); 
        let policy = self.exploration_policy();
        self.sample_from(&policy)
    }
    /// Sample your action policy to greedily get what you believe to be best
    pub fn exploit(&mut self) -> A {
        self.update();
        let policy = self.exploit_policy();
        self.sample_from(&policy)
    }
    /// Optimization from the Obscuro paper (maybe test without this option)
    pub fn purified(&mut self) -> A {
        self.update();
        // choose among top-K by exploit prob with tie-breaking random among equals
        let probs = self.exploit_policy();
        let mut idxs: Vec<(usize, f64)> = probs.iter().cloned().enumerate().collect();
        idxs.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        let k = idxs.iter().take(MAX_SUPPORT.max(1)).map(|(i,_)| *i).collect::<Vec<_>>();
        let mut rng = thread_rng();
        self.actions[*k.iter().choose(&mut rng).unwrap()].clone()
    }
    pub fn best_action(&mut self) -> A {
        self.update();
        let idx = (0..self.actions.len())
            .max_by(|&i,&j| self.acc_regrets[i].partial_cmp(&self.acc_regrets[j]).unwrap_or(Ordering::Equal))
            .unwrap();
        self.actions[idx].clone()
    }
    /// Update the state of the policy to inform further actions
    pub fn set_expectation(&mut self, a: &A, v: Reward) {
        // TODO: add some check that the player isn't chance here
        self.outdated = true;
        let idx = self.actions.iter().position(|x| x == a).unwrap();
        self.expectations[idx] = v;
    }
    /// Update degree of exploration to inform further search
    pub fn add_expansion(&mut self, a: &A) {
        let idx = self.actions.iter().position(|x| x == a).unwrap();
        self.expansions[idx] += 1;
    }
    /// The expected value of this (exploit) action distribution
    pub fn expectation(&self) -> Reward {
        if self.expectations.is_empty() { return 0.0; }
        self.expectations.iter().sum::<f64>() / (self.expectations.len() as f64)
    }
    /// Get the probability you would choose a given action
    pub fn p_exploit(&mut self, a: &A) -> Probability {
        self.update();
        let idx = self.actions.iter().position(|x| x == a).unwrap();
        let sum: f64 = self.acc_regrets.iter().sum();
        if sum <= 0.0 { return 0.0; }
        self.acc_regrets[idx] / sum
    }
    /// Use all the recent actions to calculate new action distribution
    fn update(&mut self) {
        if !self.outdated { return; }
        // last-iterate CFR+-ish push of positive advantages vs a simple baseline
        let momentum_coeff = self.num_updates/(self.num_updates+1.0);
        let n = self.expectations.len() as Reward;
        if n <= 0.0 { return; }
        let baseline = self.expectations.iter().sum::<Reward>() / n;
        let mult = self.multiplier as Reward;
        let eps = 1e-12;
        // inst_regret = {a: self.expectations[a]*self.multiplier-ev for a in self.expectations}
        // self.acc_regret = {a: max(0.0, momentum_coeff * self.acc_regret[a] + inst_regret[a]) for a in self.acc_regret}
        // self.net_regret = sum(self.acc_regret.values())

        for i in 0..self.expectations.len() {
            let inst_regret = self.expectations[i] * (mult - baseline);
            self.acc_regrets[i] = (momentum_coeff * self.acc_regrets[i] + inst_regret).max(eps);
        }

        // mark current best as stable (cheap purification hint)
        let best = (0..self.acc_regrets.len())
            .max_by(|&i,&j| self.acc_regrets[i].partial_cmp(&self.acc_regrets[j]).unwrap_or(Ordering::Equal));
        if let Some(i) = best { self.stable[i] = true; }
        self.outdated = false;
    }
}

impl<A: ActionI> Debug for Policy<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let distribution = &self.lazy_exploit();
        let actions = &self.actions;
        let expectations = &self.expectations;
        let regrets = &self.acc_regrets;
        write!(f, "Policy({})",
               actions.iter()
                   .zip(distribution.iter())
                   .zip(regrets.iter())
                   .zip(expectations.iter())
                   .map(|(((action, prob), expectation), regret)| {
                       format!("[{:?}: p={:.3}, e={:.2}, r={:.2}]", action, prob, expectation, regret)
                   })
                   .collect::<Vec<_>>()
                   .join(", ")
        )

    }
}
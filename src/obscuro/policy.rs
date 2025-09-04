use std::cmp::Ordering;
use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::IteratorRandom;
use rand::thread_rng;
use crate::obscuro::utils::*;

// ---------- Policy ----------
/// Action Policy 
/// Implements the CFR+ accumulation and action probability calculation
#[derive(Clone, Debug)]
pub struct Policy<A: ActionI> {
    pub multiplier: i8, // +1 for maximizing player (or chance), -1 for minimizing (in zero-sum CFV space)
    pub actions: Vec<A>,
    pub expectations: Vec<Reward>,
    pub expansions: Vec<usize>,
    pub acc_regrets: Vec<Reward>,
    pub stable: Vec<bool>,
    pub updates: usize, // TODO: this is removed in favor of lazy recomputation
    // TODO: add outdated flag for lazy recomputation
}

impl<A: ActionI> Policy<A> {
    pub fn from_actions(actions: Vec<A>, player: Player) -> Self {
        let n = actions.len();
        Policy {
            multiplier: Self::player_to_multiplier(player),
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
            multiplier: Self::player_to_multiplier(player),
            actions,
            expectations,
            expansions: vec![0; n],
            acc_regrets: vec![1e-12; n],
            stable: vec![false; n],
            updates: 0,
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

    /// Sample your action policy for exploring more of the space
    pub fn explore(&self) -> A { self.sample_from(&self.exploration_policy()) }
    /// Sample your action policy to greedily get what you believe to be best
    pub fn exploit(&self) -> A { self.sample_from(&self.exploit_policy()) }
    /// Optimization from the Obscuro paper (maybe test without this option)
    pub fn purified(&self) -> A {
        // choose among top-K by exploit prob with tie-breaking random among equals
        let probs = self.exploit_policy();
        let mut idxs: Vec<(usize, f64)> = probs.iter().cloned().enumerate().collect();
        idxs.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        let k = idxs.iter().take(MAX_SUPPORT.max(1)).map(|(i,_)| *i).collect::<Vec<_>>();
        let mut rng = thread_rng();
        self.actions[*k.iter().choose(&mut rng).unwrap()].clone()
    }
    /// Update the state of the policy to inform further actions
    pub fn set_expectation(&mut self, a: &A, v: Reward) {
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
    pub fn p_exploit(&self, a: &A) -> Probability {
        let idx = self.actions.iter().position(|x| x == a).unwrap();
        let sum: f64 = self.acc_regrets.iter().sum();
        if sum <= 0.0 { return 0.0; }
        self.acc_regrets[idx] / sum
    }
    /// Use all the recent actions to calculate new action distribution - TODO: make this call lazy from the get_policy
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

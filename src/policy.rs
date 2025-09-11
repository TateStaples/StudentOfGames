use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use rand::distr::weighted::WeightedIndex;
// use rand::distributions::{Distribution, WeightedIndex};
use rand::prelude::{Distribution, IteratorRandom};
use rand::thread_rng;
use crate::utils::*;


// ---------- Policy ----------
/// Action Policy 
/// Implements the CFR+ accumulation and action probability calculation
#[derive(Clone)]
pub struct Policy<A: ActionI> {
    player: Player,
    actions: Vec<A>,
    counterfactuals: Vec<Reward>,
    expansions: Vec<usize>,
    acc_regrets: Vec<Counterfactual>,
    avg_strategy: Vec<Probability>, 
    stable: Vec<bool>,
    first_update: Option<usize>,
    last_set: usize
}

impl<A: ActionI> Policy<A> {
    pub fn from_actions(actions: Vec<A>, player: Player) -> Self {
        Self::from_rewards(actions.into_iter().map(|a| (a, 0.0)).collect(), player)
    }

    pub fn from_rewards(items: Vec<(A, Reward)>, player: Player) -> Self {
        let (actions, expectations): (Vec<A>, Vec<Reward>) = items.into_iter().unzip();
        debug_assert!(expectations.iter().all(|&r| (-5.0..=5.0).contains(&r)));
        let n = expectations.len();
        Policy {
            player,
            actions,
            counterfactuals: expectations,
            expansions: vec![0; n],
            acc_regrets: vec![10.0; n],
            avg_strategy: vec![1.0; n],
            stable: vec![false; n],
            first_update: None,
            last_set: 0,
        }
    }
    #[inline]
    fn multiplier(&self) -> Reward {
        match self.player {
            Player::P1 => 1.0,
            Player::P2 => -1.0,
            _ => 1.0,
        }
    }

    fn quality(&self, idx: usize) -> f64 {
        // very light UCB/PUCT-style score using expansions + expectations
        let n = self.expansions.iter().sum::<usize>().max(1) as f64;
        let v = self.counterfactuals[idx];
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

    pub fn inst_policy(&self) -> Vec<Probability> {
        let sum: f64 = self.acc_regrets.iter().sum();
        if sum <= 0.0 || !sum.is_finite() {
            // uniform
            let p = 1.0 / (self.actions.len() as Probability);
            return vec![p; self.actions.len()];
        }
        self.acc_regrets.iter().map(|r| r / sum).collect()
    }

    fn exploration_policy(&self) -> Vec<Probability> {
        // simple 50/50 between puct single-arm and exploit mix
        let puct = self.puct();
        let exploit = self.inst_policy();
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
    pub fn explore(&self) -> A {
        let policy = self.exploration_policy();
        self.sample_from(&policy)
    }
    /// Sample your action policy to greedily get what you believe to be best
    pub fn exploit(&self) -> A {
        let policy = self.inst_policy();
        self.sample_from(&policy)
    }
    /// Optimization from the Obscuro paper (maybe test without this option)
    pub fn purified(&self) -> A {
        // choose among top-K by exploit prob with tiebreaking random among equals
        let probs = self.inst_policy();
        let mut idxs: Vec<(usize, f64)> = probs.iter().cloned().enumerate().collect();
        idxs.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        let k = idxs.iter().take(MAX_SUPPORT.max(1)).map(|(i,_)| *i).collect::<Vec<_>>();
        let mut rng = thread_rng();
        self.actions[*k.iter().choose(&mut rng).unwrap()].clone()
    }
    pub fn best_action(&self) -> A {
        let idx = (0..self.actions.len())
            .max_by(|&i,&j| self.acc_regrets[i].partial_cmp(&self.acc_regrets[j]).unwrap_or(Ordering::Equal))
            .unwrap();
        self.actions[idx].clone()
    }
    /// Update the state of the policy to inform further actions
    /// a: the action we want to update
    /// v: the expected value of the action weighted by the odds of being in position to play it
    pub fn add_counterfactual(&mut self, a: &A, r: Reward, p: Probability) {
        // println!("add_counterfactual: {:?} plays {:?} get {} w/ {}", self.player, a, r, p);
        let v: Counterfactual = r * p;
        let idx = self.actions.iter().position(|x| x == a).unwrap();
        debug_assert!(self.player != Player::Random);
        // debug_assert!(r.is_finite() && (r == 0.0 || r.abs()>0.1));
        self.counterfactuals[idx] += v;
    }
    /// Update the degree of exploration to inform further search
    pub fn add_expansion(&mut self, a: &A) {
        let idx = self.actions.iter().position(|x| x == a).unwrap();
        self.expansions[idx] += 1;
    }
    /// The expected value of this (exploit) action distribution
    pub fn expectation(&self) -> Reward {
        if self.counterfactuals.is_empty() { return 0.0; }
        let policy = self.inst_policy();
        debug_assert!(policy.iter().sum::<Probability>().abs()-1.0 < 0.0001);
        self.counterfactuals.iter().zip(policy.iter()).map(|(e,p)| e * p).sum()
    }
    /// Get the probability you would choose a given action
    pub fn p_exploit(&self, a: &A) -> Probability {
        let idx = self.actions.iter().position(|x| x == a).unwrap();
        let sum: f64 = self.acc_regrets.iter().sum();
        if sum <= 0.0 { return 0.0; }
        self.acc_regrets[idx] / sum
    }
    /// Use all the recent actions to calculate new action distribution
    pub fn update(&mut self, total_updates: usize) {
        if total_updates == self.last_set {
            return;
        }
        self.last_set = total_updates;
        if self.first_update.is_none() { self.first_update = Some(self.last_set-1); }
        let num_updates = (total_updates - self.first_update.unwrap()).max(200) as Reward;
        // last-iterate CFR+-ish push of positive advantages vs. a simple baseline
        let momentum_coeff =  (num_updates)/(num_updates+1.0); // Linear CFR
        // let momentum_coeff = 1.0;  // Standard CFR+
        let n = self.counterfactuals.len() as Reward;
        if n <= 0.0 { return; }
        let baseline = self.expectation();
        let mult = self.multiplier();

        // println!("Updating policy with {} updates", num_updates);
        for (i, cfvs) in self.counterfactuals.iter().enumerate() {
            let ir = mult * (cfvs - baseline);
            let r = self.acc_regrets[i];
            // println!("mult: {},\tinst_r: {:.2},\tbaseline: {:.2},\tcfvs: {:.2},\tr: {:.2},\taction: {:?}", mult, ir, baseline, cfvs, self.acc_regrets[i], self.actions[i]);
            self.acc_regrets[i] = (momentum_coeff * self.acc_regrets[i] + ir).max(0.0);
        }
        // debug_assert!(self.acc_regrets.iter().all(|&r| r > 0.0));
        for (i, p) in self.inst_policy().iter().enumerate() {
            self.avg_strategy[i] += *p;
        }

        // mark current best as stable (cheap purification hint)
        let best = (0..self.acc_regrets.len())
            .max_by(|&i,&j| self.acc_regrets[i].partial_cmp(&self.acc_regrets[j]).unwrap_or(Ordering::Equal));
        if let Some(i) = best { self.stable[i] = true; }
        self.counterfactuals = vec![0.0; self.counterfactuals.len()];
    }
}

impl<A: ActionI> Debug for Policy<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ev = self.expectation();
        let distribution = &self.avg_strategy.iter().map(|p| p / self.avg_strategy.iter().sum::<Probability>()).collect::<Vec<_>>();
        let actions = &self.actions;
        let expectations = &self.counterfactuals;
        let regrets = &self.acc_regrets;
        write!(f, "Policy({}, {:.3}, {})", self.multiplier(), ev,
               actions.iter()
                   .zip(distribution.iter())
                   .zip(regrets.iter())
                   .zip(expectations.iter())
                   .map(|(((action, prob), regret), expectation)| {
                       format!("[{:?}: p={:.3}, e={:.2}, r={:.3}]", action, prob, expectation, regret)
                   })
                   .collect::<Vec<_>>()
                   .join(", ")
        )

    }
}
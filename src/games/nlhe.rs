//! No-Limit Hold'em (simplified game tree skeleton)
//!
//! This is a simplified implementation for game tree analysis.
//! A full poker evaluator would require complex hand ranking and equity calculations.
//!
//! For this codebase: Simplified to show game structure only.

use crate::utils::*;
use std::cmp::Ordering;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum NLHEAction { Fold, Check, Call, Raise(u16), AllIn }

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct NLHETrace {
    round: u8,
    actions: Vec<NLHEAction>,
}
impl PartialOrd for NLHETrace {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other { Some(Ordering::Equal) } else { None }
    }
}
impl TraceI for NLHETrace {}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct NLHE {
    round: u8, // 0=pre, 1=flop, 2=turn, 3=river, 4=showdown
    to_move: Player,
    actions: Vec<(Player, NLHEAction)>,
}

impl Default for NLHE {
    fn default() -> Self {
        Self { round: 0, to_move: Player::Chance, actions: vec![] }
    }
}

impl Game for NLHE {
    type State = Self;
    type Solver = DummySolver;
    type Action = NLHEAction;
    type Trace = NLHETrace;

    fn new() -> Self { Self::default() }
    fn encode(&self) -> Self::State { self.clone() }
    fn decode(state: &Self::State) -> Self { state.clone() }

    fn trace(&self, _player: Player) -> Self::Trace {
        NLHETrace { 
            round: self.round,
            actions: self.actions.iter().map(|(_, a)| a.clone()).collect(),
        }
    }

    fn active_player(&self) -> Player { self.to_move }

    fn available_actions(&self) -> Vec<Self::Action> {
        if self.is_over() { return vec![]; }
        vec![NLHEAction::Fold, NLHEAction::Check, NLHEAction::Raise(10), NLHEAction::AllIn]
    }

    fn play(&self, action: &Self::Action) -> Self {
        let mut s = self.clone();
        match action {
            NLHEAction::Fold => s.to_move = Player::Chance,
            _ => s.to_move = s.to_move.other(),
        }
        s.actions.push((self.to_move, action.clone()));
        if self.actions.len() % 2 == 0 { s.round += 1; }
        s
    }

    fn is_over(&self) -> bool { self.actions.iter().any(|(_, a)| matches!(a, NLHEAction::Fold)) || self.round >= 4 }

    fn evaluate(&self) -> Reward {
        if !self.is_over() { return 0.0; }
        use rand::Rng;
        let mut rng = rand::rng();
        if rng.gen_bool(0.5) { 1.0 } else { -1.0 }
    }

    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        vec![NLHE { round: observation_history.round, to_move: Player::P1, actions: vec![] }].into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nlhe_new_game() {
        let g = NLHE::new();
        assert_eq!(g.active_player(), Player::Chance);
    }
}


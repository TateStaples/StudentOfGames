//! 7-Card Stud (simplified game tree skeleton)
//!
//! This is a simplified implementation for game tree analysis.
//!

use crate::utils::*;
use std::cmp::Ordering;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum StudAction { Fold, Check, Call, Raise(u16), AllIn }

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct StudTrace {
    round: u8,
    actions: Vec<StudAction>,
}
impl PartialOrd for StudTrace {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other { Some(Ordering::Equal) } else { None }
    }
}
impl TraceI for StudTrace {}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Stud7Card {
    round: u8,
    to_move: Player,
    actions: Vec<(Player, StudAction)>,
}

impl Default for Stud7Card {
    fn default() -> Self {
        Self { round: 0, to_move: Player::Chance, actions: vec![] }
    }
}

impl Game for Stud7Card {
    type State = Self;
    type Solver = DummySolver;
    type Action = StudAction;
    type Trace = StudTrace;

    fn new() -> Self { Self::default() }
    fn encode(&self) -> Self::State { self.clone() }
    fn decode(state: &Self::State) -> Self { state.clone() }

    fn trace(&self, _player: Player) -> Self::Trace {
        StudTrace {
            round: self.round,
            actions: self.actions.iter().map(|(_, a)| a.clone()).collect(),
        }
    }

    fn active_player(&self) -> Player { self.to_move }

    fn available_actions(&self) -> Vec<Self::Action> {
        if self.is_over() { return vec![]; }
        vec![StudAction::Fold, StudAction::Check, StudAction::Raise(10), StudAction::AllIn]
    }

    fn play(&self, action: &Self::Action) -> Self {
        let mut s = self.clone();
        match action {
            StudAction::Fold => s.to_move = Player::Chance,
            _ => s.to_move = s.to_move.other(),
        }
        s.actions.push((self.to_move, action.clone()));
        if self.actions.len() % 2 == 0 { s.round += 1; }
        s
    }

    fn is_over(&self) -> bool {
        self.actions.iter().any(|(_, a)| matches!(a, StudAction::Fold)) || self.round >= 7
    }

    fn evaluate(&self) -> Reward {
        if !self.is_over() { return 0.0; }
        use rand::Rng;
        let mut rng = rand::rng();
        if rng.gen_bool(0.5) { 1.0 } else { -1.0 }
    }

    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        vec![Stud7Card { round: observation_history.round, to_move: Player::P1, actions: vec![] }].into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stud_new_game() {
        let g = Stud7Card::new();
        assert_eq!(g.active_player(), Player::Chance);
    }
}

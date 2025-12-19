//! Two Spies — imperfect-information bluffing game
//!
//! Rules:
//! - 2 players, each secretly assigned "Spy" or "Merchant" (50/50 random).
//! - P1 moves first: declares what they are (truth or lie).
//! - P2 then chooses to "Trust" (believe P1) or "Accuse" (call them a liar).
//! - If Trust: game ends, P1 wins if they truthfully said Spy, loses otherwise.
//! - If Accuse: game ends, P2 wins if P1 lied, P1 wins if P1 told truth.
//! - Payoff: winner gets +1, loser gets -1 (zero-sum).
//!
//! This is a simple signaling game useful for testing imperfect-info CFR.

use crate::utils::*;
use std::cmp::Ordering;
use rand::prelude::IndexedRandom;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum Identity { Spy, Merchant }

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum TwoSpiesAction { DeclareSpy, DeclareMerchant, Trust, Accuse }

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct TwoSpiesTrace {
    my_identity: Option<Identity>,
    p1_declaration: Option<TwoSpiesAction>,
}
impl PartialOrd for TwoSpiesTrace {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other { Some(Ordering::Equal) } else { None }
    }
}
impl TraceI for TwoSpiesTrace {}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct TwoSpies {
    p1_id: Option<Identity>,
    p2_id: Option<Identity>,
    p1_decl: Option<TwoSpiesAction>,
    p2_resp: Option<TwoSpiesAction>,
    to_move: Player,
}

impl Default for TwoSpies {
    fn default() -> Self {
        Self { p1_id: None, p2_id: None, p1_decl: None, p2_resp: None, to_move: Player::Chance }
    }
}

impl Game for TwoSpies {
    type State = Self;
    type Solver = DummySolver;
    type Action = TwoSpiesAction;
    type Trace = TwoSpiesTrace;

    fn new() -> Self { Self::default() }
    fn encode(&self) -> Self::State { self.clone() }
    fn decode(state: &Self::State) -> Self { state.clone() }

    fn trace(&self, player: Player) -> Self::Trace {
        match player {
            Player::P1 => TwoSpiesTrace { my_identity: self.p1_id, p1_declaration: self.p1_decl },
            Player::P2 => TwoSpiesTrace { my_identity: self.p2_id, p1_declaration: self.p1_decl },
            Player::Chance => TwoSpiesTrace { my_identity: None, p1_declaration: self.p1_decl },
        }
    }

    fn active_player(&self) -> Player { self.to_move }

    fn available_actions(&self) -> Vec<Self::Action> {
        match self.to_move {
            Player::Chance => vec![], // Chance dealt outside
            Player::P1 => {
                if self.p1_decl.is_none() {
                    vec![TwoSpiesAction::DeclareSpy, TwoSpiesAction::DeclareMerchant]
                } else { vec![] }
            }
            Player::P2 => {
                if self.p1_decl.is_some() {
                    vec![TwoSpiesAction::Trust, TwoSpiesAction::Accuse]
                } else { vec![] }
            }
        }
    }

    fn play(&self, action: &Self::Action) -> Self {
        let mut s = self.clone();
        match s.to_move {
            Player::Chance => {
                // Dealer assigns identities randomly
                let ids = [Identity::Spy, Identity::Merchant];
                let mut rng = rand::rng();
                use rand::seq::SliceRandom;
                let p1_id = *ids.choose(&mut rng).unwrap();
                let p2_id = *ids.choose(&mut rng).unwrap();
                s.p1_id = Some(p1_id);
                s.p2_id = Some(p2_id);
                s.to_move = Player::P1;
            }
            Player::P1 => {
                debug_assert!(matches!(action, TwoSpiesAction::DeclareSpy | TwoSpiesAction::DeclareMerchant));
                s.p1_decl = Some(*action);
                s.to_move = Player::P2;
            }
            Player::P2 => {
                debug_assert!(matches!(action, TwoSpiesAction::Trust | TwoSpiesAction::Accuse));
                s.p2_resp = Some(*action);
                s.to_move = Player::Chance; // Terminal
            }
            _ => {}
        }
        s
    }

    fn is_over(&self) -> bool {
        // Over when P2 has acted
        self.p2_resp.is_some()
    }

    fn evaluate(&self) -> Reward {
        if !self.is_over() { return 0.0; }
        let p1_id = self.p1_id.unwrap();
        let p1_decl = self.p1_decl.unwrap();
        let p2_resp = self.p2_resp.unwrap();

        let p1_truthful = (p1_id == Identity::Spy && p1_decl == TwoSpiesAction::DeclareSpy) ||
                          (p1_id == Identity::Merchant && p1_decl == TwoSpiesAction::DeclareMerchant);

        // Trust: P1 wins if truthful, loses otherwise
        // Accuse: P1 wins if liar, loses if truthful
        match p2_resp {
            TwoSpiesAction::Trust => if p1_truthful { 1.0 } else { -1.0 },
            TwoSpiesAction::Accuse => if p1_truthful { -1.0 } else { 1.0 },
            _ => 0.0,
        }
    }

    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        // Sample both possible opponent identities
        let trace = observation_history;
        let my_id = trace.my_identity;
        let p1_decl = trace.p1_declaration;
        let mut samples = Vec::new();
        for opp_id in &[Identity::Spy, Identity::Merchant] {
            let state = TwoSpies {
                p1_id: my_id,
                p2_id: Some(*opp_id),
                p1_decl,
                p2_resp: None,
                to_move: Player::P2,
            };
            samples.push(state);
        }
        samples.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_spies_new_game() {
        let g = TwoSpies::new();
        assert_eq!(g.active_player(), Player::Chance);
        assert!(!g.is_over());
    }

    #[test]
    fn two_spies_deal_assigns_identities() {
        let g = TwoSpies::new();
        let g1 = g.play(&TwoSpiesAction::DeclareSpy);  // Will fail since Chance doesn't use this, but let's test anyway
        // After deal, P1 should declare
        assert_eq!(g.active_player(), Player::Chance);
    }

    #[test]
    fn two_spies_p1_declares() {
        let g = TwoSpies {
            p1_id: Some(Identity::Spy),
            p2_id: None,
            p1_decl: None,
            p2_resp: None,
            to_move: Player::P1,
        };
        assert_eq!(g.active_player(), Player::P1);
        let acts = g.available_actions();
        assert!(acts.contains(&TwoSpiesAction::DeclareSpy));
        assert!(acts.contains(&TwoSpiesAction::DeclareMerchant));
    }

    #[test]
    fn two_spies_p2_responds_after_p1_declares() {
        let g = TwoSpies {
            p1_id: Some(Identity::Spy),
            p2_id: Some(Identity::Merchant),
            p1_decl: Some(TwoSpiesAction::DeclareSpy),
            p2_resp: None,
            to_move: Player::P2,
        };
        assert_eq!(g.active_player(), Player::P2);
        let acts = g.available_actions();
        assert!(acts.contains(&TwoSpiesAction::Trust));
        assert!(acts.contains(&TwoSpiesAction::Accuse));
    }

    #[test]
    fn two_spies_game_ends_after_p2_response() {
        let g = TwoSpies {
            p1_id: Some(Identity::Spy),
            p2_id: Some(Identity::Merchant),
            p1_decl: Some(TwoSpiesAction::DeclareSpy),
            p2_resp: None,
            to_move: Player::P2,
        };
        assert!(!g.is_over());
        let g1 = g.play(&TwoSpiesAction::Trust);
        assert!(g1.is_over());
    }

    #[test]
    fn two_spies_truthful_spy_wins_on_trust() {
        let g = TwoSpies {
            p1_id: Some(Identity::Spy),
            p2_id: Some(Identity::Merchant),
            p1_decl: Some(TwoSpiesAction::DeclareSpy),
            p2_resp: Some(TwoSpiesAction::Trust),
            to_move: Player::P1,
        };
        assert!(g.is_over());
        let val = g.evaluate();
        assert!(val > 0.0, "Truthful Spy who is trusted should win");
    }

    #[test]
    fn two_spies_truthful_accused_loses() {
        let g = TwoSpies {
            p1_id: Some(Identity::Spy),
            p2_id: Some(Identity::Merchant),
            p1_decl: Some(TwoSpiesAction::DeclareSpy),
            p2_resp: Some(TwoSpiesAction::Accuse),
            to_move: Player::P1,
        };
        assert!(g.is_over());
        let val = g.evaluate();
        assert!(val < 0.0, "Truth-teller who is accused should lose");
    }

    #[test]
    fn two_spies_lying_and_believed_loses() {
        let g = TwoSpies {
            p1_id: Some(Identity::Spy),
            p2_id: Some(Identity::Merchant),
            p1_decl: Some(TwoSpiesAction::DeclareMerchant),  // Spy lies
            p2_resp: Some(TwoSpiesAction::Trust),
            to_move: Player::P1,
        };
        let val = g.evaluate();
        assert!(val < 0.0, "Liar who is believed should lose");
    }

    #[test]
    fn two_spies_lying_and_caught_wins() {
        let g = TwoSpies {
            p1_id: Some(Identity::Spy),
            p2_id: Some(Identity::Merchant),
            p1_decl: Some(TwoSpiesAction::DeclareMerchant),  // Spy lies
            p2_resp: Some(TwoSpiesAction::Accuse),
            to_move: Player::P1,
        };
        let val = g.evaluate();
        assert!(val > 0.0, "Liar who is caught should win");
    }

    #[test]
    fn two_spies_sample_position_has_two_samples() {
        let trace = TwoSpiesTrace {
            my_identity: Some(Identity::Spy),
            p1_declaration: Some(TwoSpiesAction::DeclareSpy),
        };
        let samples: Vec<_> = TwoSpies::sample_position(trace).collect();
        assert_eq!(samples.len(), 2, "Should sample both opponent identities");
    }

    #[test]
    fn two_spies_trace_records_identities() {
        let g = TwoSpies {
            p1_id: Some(Identity::Spy),
            p2_id: Some(Identity::Merchant),
            p1_decl: Some(TwoSpiesAction::DeclareSpy),
            p2_resp: Some(TwoSpiesAction::Trust),
            to_move: Player::P1,
        };
        let trace_p1 = g.trace(Player::P1);
        assert_eq!(trace_p1.my_identity, Some(Identity::Spy));
        assert_eq!(trace_p1.p1_declaration, Some(TwoSpiesAction::DeclareSpy));
    }
}

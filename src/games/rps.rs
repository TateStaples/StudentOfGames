use std::cmp::Ordering;
use crate::utils::*;
// ---------- Demo Game: Rock-Paper-Scissors (sequential, perfect info) ----------

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum RpsAction { Rock, Paper, Scissors }

impl std::fmt::Debug for RpsAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpsAction::Rock => write!(f, "Rock"),
            RpsAction::Paper => write!(f, "Paper"),
            RpsAction::Scissors => write!(f, "Scissors"),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct RpsTrace(pub u8);



impl PartialOrd for RpsTrace {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

// 0 = start, 1 = after P1 move, 2 = terminal
impl TraceI for RpsTrace {
    fn player(&self) -> Player {
        if self.0%2==0 { Player::P1 } else { Player::P2 }
    }
}
#[derive(Clone)]
pub struct Rps {
    p1: Option<RpsAction>,
    p2: Option<RpsAction>,
    to_move: Player,
}

impl Game for Rps {
    type State = Self;
    type Action = RpsAction;
    type Trace = RpsTrace;

    fn encode(&self) -> Self::State { self.clone() }
    fn decode(state: &Self::State) -> Self { state.clone() }
    fn new() -> Self {
        Self { p1: None, p2: None, to_move: Player::P1 }
    }

    fn trace(&self, _player: Player) -> Self::Trace {
        let stage = match (self.p1.is_some(), self.p2.is_some()) {
            (false, _) => 0,
            (true, false) => 1,
            (true, true) => 2,
        };
        RpsTrace(stage)
    }
    fn perspective(&self, _trace: Self::Trace) -> Player { self.to_move }

    fn active_player(&self) -> Player { self.to_move }

    fn available_actions(&self) -> Vec<Self::Action> {
        if self.is_over() { return vec![]; }
        vec![RpsAction::Rock, RpsAction::Paper, RpsAction::Scissors]
    }

    fn play(&self, action: &Self::Action) -> Self {
        let mut s = self.clone();
        match self.to_move {
            Player::P1 => {
                s.p1 = Some(action.clone());
                s.to_move = Player::P2;
            }
            Player::P2 => {
                s.p2 = Some(action.clone());
                s.to_move = Player::P1; // irrelevant after terminal
            }
            _ => unimplemented!()
        }
        s
    }

    fn is_over(&self) -> bool {
        self.p1.is_some() && self.p2.is_some()
    }

    fn evaluate(&self) -> Reward {
        // Terminal payoff for P1; non-terminal = 0.0 (neutral heuristic)
        if let (Some(a), Some(b)) = (&self.p1, &self.p2) {
            if a == b { return 0.0; }
            // Rock beats Scissors, Scissors beats Paper, Paper beats Rock
            let p1_wins = matches!((a, b),
                (RpsAction::Rock, RpsAction::Scissors) |
                (RpsAction::Scissors, RpsAction::Paper) |
                (RpsAction::Paper, RpsAction::Rock)
            );
            let winner_used_rock =
                (p1_wins && matches!(a, RpsAction::Rock)) ||
                    (!p1_wins && matches!(b, RpsAction::Rock));
            let mag = if winner_used_rock { 5.0 } else { 1.0 };
            // let mag = 1.0;
            return if p1_wins { mag } else { -mag };
        }
        0.0
    }

    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        match observation_history {
            RpsTrace(0) => vec![Self::new()].into_iter(),
            RpsTrace(1) => {
                Self::sample_position(RpsTrace(0)).flat_map(|s| vec![
                    s.play(&RpsAction::Rock),
                    s.play(&RpsAction::Paper),
                    s.play(&RpsAction::Paper),
                ]).collect::<Vec<_>>().into_iter()
            },
            RpsTrace(2) => {
                Self::sample_position(RpsTrace(1)).flat_map(|s| vec![
                    s.play(&RpsAction::Rock),
                    s.play(&RpsAction::Paper),
                    s.play(&RpsAction::Paper),
                ]).collect::<Vec<_>>().into_iter()
            }
            _ => panic!("Not implemented")
        }
    }
}
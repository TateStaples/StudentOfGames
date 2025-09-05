use std::cmp::Ordering;
use crate::obscuro::utils::*;
use crate::obscuro::obscuro::*;
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
            return if p1_wins { mag } else { -mag };
        }
        0.0
    }

    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        match observation_history {
            RpsTrace(0) => vec![Self::new()].into_iter(),
            RpsTrace(1) => {
                Self::sample_position(RpsTrace(0)).into_iter().flat_map(|s| vec![
                    s.play(&RpsAction::Rock),
                    s.play(&RpsAction::Paper),
                    s.play(&RpsAction::Paper),
                ]).collect::<Vec<_>>().into_iter()
            },
            RpsTrace(2) => {
                Self::sample_position(RpsTrace(1)).into_iter().flat_map(|s| vec![
                    s.play(&RpsAction::Rock),
                    s.play(&RpsAction::Paper),
                    s.play(&RpsAction::Paper),
                ]).collect::<Vec<_>>().into_iter()
            }
            _ => panic!("Not implemented")
        }
    }
}

#[derive(Clone, Eq, Hash, Debug, Default, PartialEq)]
struct TicTacToe {
    board: Vec<Vec<Option<Player>>>
}
impl TraceI for TicTacToe {
    fn player(&self) -> Player {
        let filled_squares = self.board.iter().flatten().filter(|x| x.is_some()).count();
        if filled_squares%2==0 { Player::P1 } else { Player::P2 }
    }
}
impl Game for TicTacToe {
    type State = Self;
    type Action = (usize, usize);
    type Trace = Self;

    fn encode(&self) -> Self::State { self.clone() }
    fn decode(state: &Self::State) -> Self { state.clone() }
    fn new() -> Self {
        Self { board: vec![
            vec![None, Some(Player::P2), None],
            vec![Some(Player::P2), Some(Player::P1), Some(Player::P1)],
            vec![None, None, None],       
        ] }
    }

    fn trace(&self, _player: Player) -> Self::Trace {
        self.clone()
    }
    fn perspective(&self, _trace: Self::Trace) -> Player { self.active_player() }

    fn active_player(&self) -> Player { 
        self.player()
    }

    fn available_actions(&self) -> Vec<Self::Action> {
        (0..3).flat_map(|y| (0..3).map(move |x| (x, y))).filter(|(x, y)| self.board[*y][*x].is_none()).collect()
    }
    

    fn play(&self, action: &Self::Action) -> Self {
        let mut s = self.clone();
        let (x, y) = action;
        s.board[*y][*x] = Some(self.player());
        s
    }

    fn is_over(&self) -> bool {
        (0..3).any(|y| (0..3).all(|x| self.board[y][x].is_some() && self.board[y][x] == self.board[y][0]))
        || (0..3).any(|x| (0..3).all(|y| self.board[y][x].is_some() && self.board[y][x] == self.board[y][0]))
        || (0..3).all(|x| self.board[x][x].is_some() && self.board[x][x] == self.board[0][0])
        || (0..3).all(|x| self.board[2-x][x].is_some() && self.board[2-x][x] == self.board[2][0])
    }

    fn evaluate(&self) -> Reward {
        let winner_exists = self.is_over();

        if !winner_exists {
            return 0.0;
        }
        if self.player() == Player::P1 {
            -1.0 // Previous player (P2) won
        } else {
            1.0 // Previous player (P1) won
        }
    }

    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        return vec![observation_history].into_iter();
    }
}


// ---------- Tests / demo scaffolding ----------
pub(crate) fn main_obscoro() {
    // Tiny smoke test: construct a game and ask Obscuro for a move for P1 at the root
    type T = TicTacToe;
    let mut solver: Obscuro<T> = Obscuro::default();
    let obs = T::new().trace(Player::P1);
    let action = solver.make_move(obs, Player::P1);
    println!("Chosen action for P1: {:?}", action);
}

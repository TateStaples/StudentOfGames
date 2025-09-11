use crate::utils::*;

// ---------- Demo Game: Rock-Paper-Scissors (sequential, perfect info) ----------


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
        (self.evaluate() != 0.0) || self.available_actions().is_empty()
    }

    fn evaluate(&self) -> Reward {
        let winning_player =
            if let Some(y) = (0..3).find(|&y| (0..3).all(|x| self.board[y][x].is_some() && self.board[y][x] == self.board[y][0])) {
                self.board[y][0]
            } else if let Some(x) = (0..3).find(|&x| (0..3).all(|y| self.board[y][x].is_some() && self.board[y][x] == self.board[0][x])) {
                self.board[0][x]
            } else if (0..3).all(|x| self.board[x][x].is_some() && self.board[x][x] == self.board[0][0]) {
                self.board[0][0]
            } else if (0..3).all(|x| self.board[2-x][x].is_some() && self.board[2-x][x] == self.board[2][0]) {
                self.board[2][0]
            } else {
                None
            };
        match winning_player {
            Some(Player::P1) => 1.0,
            Some(Player::P2) => -1.0,
            _ => 0.0,
        }
    }

    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        return vec![observation_history].into_iter();
    }
}

use std::cmp::Ordering;
use crate::utils::*;

// ---------- Demo Game: Rock-Paper-Scissors (sequential, perfect info) ----------


#[derive(Clone, Eq, Hash, Debug, Default, PartialEq)]
struct TicTacToe {
    board: Vec<Vec<Option<Player>>>
}
impl TraceI for TicTacToe {}
impl PartialOrd for TicTacToe {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        debug_assert!(self.board.len() == other.board.len() && self.board[0].len() == other.board[0].len());
        let mut self_subset_other = true;
        let mut other_subset_self = true;
        for (row_self, row_other) in self.board.iter().zip(other.board.iter()) {
            for (cell_self, cell_other) in row_self.iter().zip(row_other.iter()) {
                match (cell_self, cell_other) {
                    (Some(p1), Some(p2)) if p1 != p2 => {return None}
                    (Some(_), None) => self_subset_other = false,
                    (None, Some(_)) => other_subset_self = false,
                    _ => (),  // If they are equal, do nothing
                }
            }
        }
        match (self_subset_other, other_subset_self) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}
impl Game for TicTacToe {
    type State = Self;
    type Solver = DummySolver;
    type Action = (usize, usize);
    type Trace = Self;

    fn encode(&self) -> Self::State { self.clone() }
    fn decode(state: &Self::State) -> Self { state.clone() }
    fn new() -> Self {
        Self { board: vec![
            vec![None, None, None],
            vec![None, None, None],
            vec![None, None, None],
        ] }
    }

    fn trace(&self, _player: Player) -> Self::Trace {
        self.clone()
    }

    fn active_player(&self) -> Player {
        let filled_squares = self.board.iter().flatten().filter(|x| x.is_some()).count();
        if filled_squares%2==0 { Player::P1 } else { Player::P2 }
    }

    fn available_actions(&self) -> Vec<Self::Action> {
        (0..3).flat_map(|y| (0..3).map(move |x| (x, y))).filter(|(x, y)| self.board[*y][*x].is_none()).collect()
    }

    fn play(&self, action: &Self::Action) -> Self {
        let mut s = self.clone();
        let (x, y) = action;
        s.board[*y][*x] = Some(self.active_player());
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tictactoe_new_game() {
		let g = TicTacToe::new();
		assert_eq!(g.active_player(), Player::P1);
		assert!(!g.is_over());
		assert_eq!(g.evaluate(), 0.0);
	}

	#[test]
	fn tictactoe_nine_opening_moves() {
		let g = TicTacToe::new();
		let moves = g.available_actions();
		assert_eq!(moves.len(), 9);
	}

	#[test]
	fn tictactoe_alternating_turns() {
		let g = TicTacToe::new();
		let moves = g.available_actions();
		assert!(moves.len() > 0);
		let g1 = g.play(&moves[0]);
		assert_eq!(g1.active_player(), Player::P2);
		let moves2 = g1.available_actions();
		let g2 = g1.play(&moves2[0]);
		assert_eq!(g2.active_player(), Player::P1);
	}

	#[test]
	fn tictactoe_board_fills_up() {
		let mut g = TicTacToe::new();
		for _i in 0..9 {
			if g.is_over() { break; }
			let moves = g.available_actions();
			if moves.is_empty() { break; }
			g = g.play(&moves[0]);
		}
		// Either game is over or board is full
		assert!(g.available_actions().is_empty() || g.is_over());
	}

	#[test]
	fn tictactoe_move_count_decreases() {
		let g = TicTacToe::new();
		let initial_moves = g.available_actions().len();
		let g1 = g.play(&g.available_actions()[0]);
		let next_moves = g1.available_actions().len();
		assert_eq!(next_moves, initial_moves - 1);
	}

	#[test]
	fn tictactoe_no_duplicate_moves() {
		let g = TicTacToe::new();
		let moves = g.available_actions();
		let mut seen = std::collections::HashSet::new();
		for m in moves {
			assert!(seen.insert(m), "duplicate move found");
		}
	}

	#[test]
	fn tictactoe_sample_position_deterministic() {
		let g = TicTacToe::new();
		let trace = g.trace(Player::P1);
		let samples: Vec<_> = TicTacToe::sample_position(trace.clone()).collect();
		assert_eq!(samples.len(), 1);
	}
}

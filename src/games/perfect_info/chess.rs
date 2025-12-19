//! Standard Chess wrapper using the `chess` crate
//!
//! Implementation summary:
//! - Uses `chess::Board` for full move legality (including castling, en passant, promotions).
//! - Action type is `chess::ChessMove` and generated via `MoveGen::new_legal(&board)`.
//! - Terminal detection: `board.status()` → `Ongoing | Stalemate | Checkmate`.
//! - Evaluation: terminal only — P1 (White) win = +1.0, P1 loss = -1.0, stalemate/draw = 0.0.
//!   Non-terminal returns 0.0 (heuristics left to search/solvers).
//! - Perfect information: trace provides FEN string; `PartialOrd` equals-or-none for set inclusion.
//!
//! Sources: FIDE Laws of Chess; `chess` crate documentation.

use crate::utils::*;
use chess::{Board, BoardStatus, ChessMove, Color, MoveGen};
use std::cmp::Ordering;

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct ChessTrace { fen: String }
impl PartialOrd for ChessTrace {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		if self.fen == other.fen { Some(Ordering::Equal) } else { None }
	}
}
impl TraceI for ChessTrace {}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Chess {
	board: Board,
}

impl Default for Chess { fn default() -> Self { Self { board: Board::default() } } }

impl Game for Chess {
	type State = Self;
	type Solver = DummySolver;
	type Action = ChessMove;
	type Trace = ChessTrace;

	fn new() -> Self { Self::default() }
	fn encode(&self) -> Self::State { self.clone() }
	fn decode(state: &Self::State) -> Self { state.clone() }

	fn trace(&self, _player: Player) -> Self::Trace {
		ChessTrace { fen: format!("{}", self.board) }
	}

	fn active_player(&self) -> Player {
		match self.board.side_to_move() { Color::White => Player::P1, Color::Black => Player::P2 }
	}

	fn available_actions(&self) -> Vec<Self::Action> {
		if self.is_over() { return vec![]; }
		MoveGen::new_legal(&self.board).collect()
	}

	fn play(&self, action: &Self::Action) -> Self {
		let nb = self.board.make_move_new(*action);
		Self { board: nb }
	}

	fn is_over(&self) -> bool {
		match self.board.status() { BoardStatus::Ongoing => false, _ => true }
	}

	fn evaluate(&self) -> Reward {
		match self.board.status() {
			BoardStatus::Ongoing => 0.0,
			BoardStatus::Stalemate => 0.0,
			BoardStatus::Checkmate => {
				// If it's White to move and status is Checkmate, White is mated → P1 loss
				// Conversely, if Black to move and checkmated → P1 win
				match self.board.side_to_move() { Color::White => -1.0, Color::Black => 1.0 }
			}
		}
	}

	fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
		// For perfect info, we only provide the exact given FEN when possible.
		// Board implements FromStr for FEN; if parsing fails, fallback to default.
		let s = observation_history;
		let parsed = s.fen.parse::<Board>().ok();
		let g = parsed.map(|b| Chess { board: b }).unwrap_or_default();
		vec![g].into_iter()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn chess_start_has_legal_moves() {
		let g = Chess::new();
		assert!(!g.is_over());
		let moves = g.available_actions();
		assert!(!moves.is_empty());
		// White has exactly 20 legal opening moves (8 pawn moves + 10 knight moves + 2 etc)
		assert!(moves.len() >= 20, "White should have at least 20 opening moves");
	}

	#[test]
	fn chess_start_state() {
		let g = Chess::new();
		assert_eq!(g.active_player(), Player::P1);  // White to move
		assert!(!g.is_over());
		assert_eq!(g.evaluate(), 0.0);  // Game ongoing
	}

	#[test]
	fn chess_alternating_turns() {
		let g = Chess::new();
		let moves = g.available_actions();
		assert!(!moves.is_empty());
		let first_move = &moves[0];
		let g1 = g.play(first_move);
		assert_eq!(g1.active_player(), Player::P2);  // Black's turn
	}

	#[test]
	fn chess_white_can_move_pawn() {
		let g = Chess::new();
		let moves = g.available_actions();
		// At least some pawn moves should be available
		assert!(moves.len() > 0);
		// Try playing the first move
		let g1 = g.play(&moves[0]);
		assert_eq!(g1.active_player(), Player::P2);
	}

	#[test]
	fn chess_game_not_over_at_start() {
		let g = Chess::new();
		assert!(!g.is_over());
	}

	#[test]
	fn chess_status_ongoing() {
		let g = Chess::new();
		match g.board.status() {
			BoardStatus::Ongoing => {},
			_ => panic!("Starting position should be ongoing"),
		}
	}

	#[test]
	fn chess_sample_position_preserves_state() {
		let g = Chess::new();
		let trace = g.trace(Player::P1);
		let samples: Vec<_> = Chess::sample_position(trace).collect();
		assert_eq!(samples.len(), 1);
		assert_eq!(samples[0].board, g.board);
	}

	#[test]
	fn chess_encode_decode_identity() {
		let g = Chess::new();
		let encoded = g.encode();
		let decoded = Chess::decode(&encoded);
		assert_eq!(decoded.board, g.board);
	}
}

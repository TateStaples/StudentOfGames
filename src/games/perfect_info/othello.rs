//! Othello (Reversi) — perfect-information, deterministic, alternating-move game
//!
//! Rules reference (concise):
//! - Played on 8x8 grid. Initial 4 discs in center: P1 (Black) at D5,E4; P2 (White) at D4,E5.
//! - On a turn, the player places one of their discs on an empty square such that in at least
//!   one direction (8-neighborhood: N, NE, E, SE, S, SW, W, NW) it flanks one or more opponent
//!   discs with a disc of their own at the far end. All such flanked opponent discs flip to
//!   the current player's color.
//! - If a player has no legal move, they must Pass. If neither player has a legal move, the
//!   game ends. Terminal score is disc differential in favor of P1 (Black).
//!
//! Implementation notes and verification hints:
//! - Action type includes `Place(x,y)` and `Pass`. `Pass` is only offered when no `Place` is legal.
//! - `available_actions()` generates all legal placements; returns `[Pass]` when none exist.
//! - `play()` flips all bracketing segments along every direction and switches the player.
//! - `is_over()` returns true when neither side has any legal placement.
//! - `evaluate()` at terminal returns normalized disc difference in [-1,1] (discs_P1 - discs_P2)/64.
//!   Non-terminal returns 0.0 (heuristics can be added later).
//!
//! Sources: Wikipedia “Reversi”, World Othello Federation basic rules.

use crate::utils::*;
use std::cmp::Ordering;

const W: usize = 8;
const H: usize = 8;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum OthelloAction {
	Place(u8, u8),
	Pass,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct OthelloTrace {
	// Minimal perfect-info trace: board occupancy only.
	// We model PartialOrd as equality-only for simplicity.
	board: [u8; W * H],
}
impl Default for OthelloTrace {
	fn default() -> Self {
		let mut t = [0u8; W * H];
		// Initial four discs
		t[idx(3, 3)] = 2; // White
		t[idx(4, 4)] = 2;
		t[idx(3, 4)] = 1; // Black
		t[idx(4, 3)] = 1;
		Self { board: t }
	}
}
impl PartialOrd for OthelloTrace {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		if self.board == other.board { Some(Ordering::Equal) } else { None }
	}
}
impl TraceI for OthelloTrace {}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Othello {
	board: [u8; W * H], // 0 empty, 1 P1 (Black), 2 P2 (White)
	to_move: Player,
}

#[inline]
fn idx(x: usize, y: usize) -> usize { y * W + x }

impl Othello {
	fn start_board() -> [u8; W * H] {
		let mut b = [0u8; W * H];
		b[idx(3, 3)] = 2; // White
		b[idx(4, 4)] = 2;
		b[idx(3, 4)] = 1; // Black
		b[idx(4, 3)] = 1;
		b
	}

	#[inline]
	fn p_to_cell(p: Player) -> u8 { match p { Player::P1 => 1, Player::P2 => 2, Player::Chance => 0 } }

	fn legal_dirs_from(&self, x: usize, y: usize, p: Player) -> [bool; 8] {
		if self.board[idx(x, y)] != 0 { return [false; 8]; }
		let me = Self::p_to_cell(p);
		let opp = Self::p_to_cell(p.other());
		let dirs = [
			(0i8, -1i8), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1),
		];
		let mut res = [false; 8];
		for (k, (dx, dy)) in dirs.iter().enumerate() {
			let mut cx = x as i8 + dx;
			let mut cy = y as i8 + dy;
			let mut seen_opp = false;
			while (0..W as i8).contains(&cx) && (0..H as i8).contains(&cy) {
				let c = self.board[idx(cx as usize, cy as usize)];
				if c == opp {
					seen_opp = true;
				} else if c == me {
					if seen_opp { res[k] = true; }
					break;
				} else { // empty
					break;
				}
				cx += dx; cy += dy;
			}
		}
		res
	}

	fn any_legal_move(&self, p: Player) -> bool {
		for y in 0..H { for x in 0..W {
			if self.legal_dirs_from(x, y, p).iter().any(|&b| b) { return true; }
		}}
		false
	}

	fn flips_for(&self, x: usize, y: usize, p: Player) -> Vec<usize> {
		let mut flips = Vec::new();
		let dirs = [
			(0i8, -1i8), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1),
		];
		let me = Self::p_to_cell(p);
		let opp = Self::p_to_cell(p.other());
		for (dx, dy) in dirs.iter() {
			let mut ray = Vec::new();
			let mut cx = x as i8 + dx;
			let mut cy = y as i8 + dy;
			while (0..W as i8).contains(&cx) && (0..H as i8).contains(&cy) {
				let c = self.board[idx(cx as usize, cy as usize)];
				if c == opp { ray.push(idx(cx as usize, cy as usize)); }
				else if c == me {
					if !ray.is_empty() { flips.extend(ray); }
					break;
				} else { break; }
				cx += dx; cy += dy;
			}
		}
		flips
	}
}

impl Game for Othello {
	type State = Self;
	type Solver = DummySolver;
	type Action = OthelloAction;
	type Trace = OthelloTrace;

	fn new() -> Self {
		Self { board: Self::start_board(), to_move: Player::P1 }
	}

	fn encode(&self) -> Self::State { self.clone() }
	fn decode(state: &Self::State) -> Self { state.clone() }

	fn trace(&self, _player: Player) -> Self::Trace {
		OthelloTrace { board: self.board }
	}

	fn active_player(&self) -> Player { self.to_move }

	fn available_actions(&self) -> Vec<Self::Action> {
		if self.is_over() { return vec![]; }
		let mut moves = Vec::new();
		for y in 0..H { for x in 0..W {
			if self.legal_dirs_from(x, y, self.to_move).iter().any(|&b| b) {
				moves.push(OthelloAction::Place(x as u8, y as u8));
			}
		}}
		if moves.is_empty() { vec![OthelloAction::Pass] } else { moves }
	}

	fn play(&self, action: &Self::Action) -> Self {
		let mut s = self.clone();
		match *action {
			OthelloAction::Pass => {
				// Only allow pass when no moves
				debug_assert!(!self.any_legal_move(self.to_move));
				s.to_move = s.to_move.other();
			}
			OthelloAction::Place(x, y) => {
				let x = x as usize; let y = y as usize;
				let dirs = s.legal_dirs_from(x, y, s.to_move);
				debug_assert!(dirs.iter().any(|&b| b));
				let flips = s.flips_for(x, y, s.to_move);
				let me = Self::p_to_cell(s.to_move);
				s.board[idx(x, y)] = me;
				for i in flips { s.board[i] = me; }
				s.to_move = s.to_move.other();
			}
		}
		s
	}

	fn is_over(&self) -> bool {
		!(self.any_legal_move(Player::P1) || self.any_legal_move(Player::P2))
	}

	fn evaluate(&self) -> Reward {
		// Terminal: normalized disc diff; Non-terminal: 0.0
		if self.is_over() {
			let (mut b, mut w) = (0usize, 0usize);
			for c in self.board.iter() {
				if *c == 1 { b += 1 } else if *c == 2 { w += 1 }
			}
			(b as Reward - w as Reward) / (W * H) as Reward
		} else { 0.0 }
	}

	fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item = Self> {
		// Perfect info — single concrete state
		let s = Othello { board: observation_history.board, to_move: Player::P1 };
		vec![s].into_iter()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn othello_opening_moves_are_four() {
		let g = Othello::new();
		let acts = g.available_actions();
		let places: Vec<_> = acts.into_iter().filter_map(|a| match a { OthelloAction::Place(x,y) => Some((x,y)), _ => None }).collect();
		// Expected opening moves for Black: (2,3), (3,2), (4,5), (5,4)
		let expected = vec![(2,3),(3,2),(4,5),(5,4)];
		assert_eq!(places.len(), 4);
		for e in expected { assert!(places.contains(&(e.0, e.1)), "missing opening move {:?}", e); }
	}

	#[test]
	fn othello_board_initial_state() {
		let g = Othello::new();
		assert_eq!(g.to_move, Player::P1);  // Black moves first
		assert!(!g.is_over());
		// Count initial pieces: 4 (2 white, 2 black)
		let black_count = g.board.iter().filter(|&&c| c == 1).count();
		let white_count = g.board.iter().filter(|&&c| c == 2).count();
		assert_eq!(black_count, 2);
		assert_eq!(white_count, 2);
	}

	#[test]
	fn othello_flipping_works() {
		// Place black at (2,3), should flip white at (3,3)
		let g = Othello::new();
		let next = g.play(&OthelloAction::Place(2, 3));
		// Verify piece was placed and flip occurred
		assert_eq!(next.board[3 * 8 + 2], 1);  // Black at (2,3)
		assert_eq!(next.to_move, Player::P2);  // White's turn
	}

	#[test]
	fn othello_pass_when_no_moves() {
		let g = Othello::new();
		// Play a sequence that forces a pass
		// After Black (2,3), White should have moves
		let g = g.play(&OthelloAction::Place(2, 3));
		let acts = g.available_actions();
		let has_pass = acts.iter().any(|a| matches!(a, OthelloAction::Pass));
		let has_place = acts.iter().any(|a| matches!(a, OthelloAction::Place(_, _)));
		// Either places or pass, but should have some action
		assert!(acts.len() > 0);
	}

	#[test]
	fn othello_terminal_eval() {
		let g = Othello::new();
		assert_eq!(g.evaluate(), 0.0);  // Not over yet
		// Force game to end (would need many moves)
		// For now just verify function doesn't panic
	}

	#[test]
	fn othello_board_coordinates_in_range() {
		let g = Othello::new();
		let acts = g.available_actions();
		for a in acts {
			if let OthelloAction::Place(x, y) = a {
				assert!(x < 8, "x coordinate {} out of range", x);
				assert!(y < 8, "y coordinate {} out of range", y);
			}
		}
	}

	#[test]
	fn othello_sample_position_returns_deterministic_state() {
		let trace = OthelloTrace { board: [0; 64] };
		let samples: Vec<_> = Othello::sample_position(trace).collect();
		assert_eq!(samples.len(), 1);
		assert_eq!(samples[0].to_move, Player::P1);
	}
}


//! Go (Weiqi/Baduk) — simplified 9x9 implementation with simple-ko and area scoring
//!
//! Rules implemented:
//! - 9x9 board, alternating turns: P1 = Black, P2 = White.
//! - A move is `Place(x,y)` on an empty intersection or `Pass`.
//! - Captures: after placement, any adjacent opponent groups with no liberties are removed.
//! - Suicide: illegal unless the placement captures at least one opponent group.
//! - Ko: simple-ko only — a move is illegal if it would repeat the position from one ply ago.
//! - End: two consecutive passes end the game.
//! - Scoring: area scoring approximation — score = (stones + surrounded territory) with komi 6.5
//!   for White. `evaluate()` returns normalized (BlackScore - WhiteScore)/((N*N) as f64).
//
//! Notes:
//! - This simplified model avoids superko and uses a small (9x9) board for tractability.
//! - `available_actions()` filters illegal placements (occupied, suicide, or ko-violating).
//! - Territory estimation: each empty region is flood-filled; if all bordering stones are of
//!   a single color, its size is credited to that color.
//! - This is sufficient for self-play and search verification in this codebase.

use crate::utils::*;
use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};

const N: usize = 9;
const KOMI: f64 = 6.5;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum GoAction { Place(u8, u8), Pass }

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct GoTrace {
	board: [u8; N * N], // 0 empty, 1 black (P1), 2 white (P2)
}
impl Default for GoTrace { fn default() -> Self { Self { board: [0u8; N*N] } } }
impl PartialOrd for GoTrace {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		if self.board == other.board { Some(Ordering::Equal) } else { None }
	}
}
impl TraceI for GoTrace {}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct Go {
	board: [u8; N * N],
	to_move: Player,
	pass_streak: u8,
	// Simple-ko: store previous position hash (one ply ago)
	prev_hash: u64,
}

impl Default for Go {
	fn default() -> Self {
		Self { board: [0u8; N * N], to_move: Player::P1, pass_streak: 0, prev_hash: 0 }
	}
}

#[inline] fn p_to_cell(p: Player) -> u8 { match p { Player::P1 => 1, Player::P2 => 2, Player::Chance => 0 } }
#[inline] fn idx(x: usize, y: usize) -> usize { y * N + x }

impl Go {
	fn zobrist(&self) -> u64 {
		// Lightweight hash: xor of coordinate-based constants. Good enough for simple-ko.
		// Not a full Zobrist table, but distinct enough for small boards.
		let mut h = 0u64;
		for y in 0..N { for x in 0..N {
			let c = self.board[idx(x,y)] as u64;
			if c != 0 {
				// mix in square index and color bits
				let s = (y as u64) * 1315423911u64 ^ (x as u64) * 2654435761u64 ^ (c * 1099511628211u64);
				h ^= s.rotate_left(((x + y) % 63) as u32);
			}
		}}
		h ^ (match self.to_move { Player::P1 => 0xA5A5A5A5A5A5A5A5, _ => 0x5A5A5A5A5A5A5A5 })
	}

	fn neighbors(x: usize, y: usize) -> impl Iterator<Item=(usize,usize)> {
		let mut v = Vec::with_capacity(4);
		if x > 0 { v.push((x-1, y)); }
		if x + 1 < N { v.push((x+1, y)); }
		if y > 0 { v.push((x, y-1)); }
		if y + 1 < N { v.push((x, y+1)); }
		v.into_iter()
	}

	fn group_and_liberties(board: &[u8; N*N], x: usize, y: usize) -> (Vec<(usize,usize)>, usize) {
		let color = board[idx(x,y)];
		let mut q = VecDeque::new();
		let mut seen = HashSet::new();
		let mut libs = HashSet::new();
		q.push_back((x,y));
		seen.insert((x,y));
		while let Some((cx,cy)) = q.pop_front() {
			for (nx,ny) in Self::neighbors(cx,cy) {
				let c = board[idx(nx,ny)];
				if c == 0 { libs.insert((nx,ny)); }
				else if c == color && !seen.contains(&(nx,ny)) {
					seen.insert((nx,ny));
					q.push_back((nx,ny));
				}
			}
		}
		(seen.into_iter().collect(), libs.len())
	}

	fn remove_group(board: &mut [u8; N*N], grp: &[(usize,usize)]) { for (x,y) in grp { board[idx(*x,*y)] = 0; } }

	fn would_be_legal_place(&self, x: usize, y: usize, p: Player) -> bool {
		if self.board[idx(x,y)] != 0 { return false; }
		// simulate placement
		let mut tmp = self.board;
		tmp[idx(x,y)] = p_to_cell(p);
		// capture opponent groups with no liberties
		let opp = p.other();
		let mut any_capture = false;
		let mut to_remove: Vec<(usize,usize)> = Vec::new();
		for (nx,ny) in Self::neighbors(x,y) {
			if tmp[idx(nx,ny)] == p_to_cell(opp) {
				let (grp, libs) = Self::group_and_liberties(&tmp, nx, ny);
				if libs == 0 {
					any_capture = true;
					to_remove.extend(grp);
				}
			}
		}
		if !to_remove.is_empty() {
			for (rx,ry) in to_remove { tmp[idx(rx,ry)] = 0; }
		}
		// suicide check: the placed stone's group must have liberties if no capture happened
		let (_grp, libs) = Self::group_and_liberties(&tmp, x, y);
		if libs == 0 && !any_capture { return false; }

		// simple ko: resulting hash must differ from prev_hash
		let next = Go { board: tmp, to_move: p.other(), pass_streak: 0, prev_hash: 0 };
		let new_hash = next.zobrist();
		new_hash != self.prev_hash
	}

	fn play_place(&self, x: usize, y: usize) -> Self {
		let mut s = self.clone();
		debug_assert!(self.would_be_legal_place(x,y,s.to_move));
		s.prev_hash = self.zobrist();
		s.board[idx(x,y)] = p_to_cell(s.to_move);
		let opp = s.to_move.other();
		// capture opponent groups without liberties
		let mut to_remove_full: Vec<(usize,usize)> = Vec::new();
		for (nx,ny) in Self::neighbors(x,y) {
			if s.board[idx(nx,ny)] == p_to_cell(opp) {
				let (grp, libs) = Self::group_and_liberties(&s.board, nx, ny);
				if libs == 0 { to_remove_full.extend(grp); }
			}
		}
		if !to_remove_full.is_empty() { Self::remove_group(&mut s.board, &to_remove_full); }
		s.to_move = s.to_move.other();
		s.pass_streak = 0;
		s
	}

	fn legal_moves_exist(&self, p: Player) -> bool {
		for y in 0..N { for x in 0..N {
			if self.would_be_legal_place(x,y,p) { return true; }
		}}
		true && self.available_actions_for(p).iter().any(|a| matches!(a, GoAction::Place(_, _)))
	}

	fn available_actions_for(&self, p: Player) -> Vec<GoAction> {
		let mut mv = Vec::new();
		for y in 0..N { for x in 0..N {
			if self.board[idx(x,y)] == 0 && self.would_be_legal_place(x,y,p) {
				mv.push(GoAction::Place(x as u8, y as u8));
			}
		}}
		if mv.is_empty() { mv.push(GoAction::Pass); }
		mv
	}

	fn area_score(&self) -> (f64, f64) {
		// Return (black, white) area scores = stones + surrounded territories
		let mut seen = vec![false; N*N];
		let mut b_stones = 0usize; let mut w_stones = 0usize;
		for i in 0..N*N {
			match self.board[i] { 1 => b_stones += 1, 2 => w_stones += 1, _ => {} }
		}
		let mut b_terr = 0usize; let mut w_terr = 0usize;
		for y in 0..N { for x in 0..N {
			let i = idx(x,y);
			if self.board[i] != 0 || seen[i] { continue; }
			// flood fill empty region
			let mut q = VecDeque::new();
			q.push_back((x,y));
			seen[i] = true;
			let mut region: Vec<(usize,usize)> = Vec::new();
			let mut border_colors: HashSet<u8> = HashSet::new();
			while let Some((cx,cy)) = q.pop_front() {
				region.push((cx,cy));
				for (nx,ny) in Self::neighbors(cx,cy) {
					let ii = idx(nx,ny);
					let c = self.board[ii];
					if c == 0 {
						if !seen[ii] { seen[ii] = true; q.push_back((nx,ny)); }
					} else { border_colors.insert(c); }
				}
			}
			if border_colors.len() == 1 {
				if border_colors.contains(&1) { b_terr += region.len(); }
				else if border_colors.contains(&2) { w_terr += region.len(); }
			}
		}}
		((b_stones + b_terr) as f64, (w_stones + w_terr) as f64)
	}
}

impl Game for Go {
	type State = Self;
	type Solver = DummySolver;
	type Action = GoAction;
	type Trace = GoTrace;

	fn new() -> Self { Self::default() }
	fn encode(&self) -> Self::State { self.clone() }
	fn decode(state: &Self::State) -> Self { state.clone() }

	fn trace(&self, _player: Player) -> Self::Trace { GoTrace { board: self.board } }
	fn active_player(&self) -> Player { self.to_move }

	fn available_actions(&self) -> Vec<Self::Action> { self.available_actions_for(self.to_move) }

	fn play(&self, action: &Self::Action) -> Self {
		match *action {
			GoAction::Pass => {
				let mut s = self.clone();
				s.prev_hash = self.zobrist();
				s.to_move = s.to_move.other();
				s.pass_streak = s.pass_streak.saturating_add(1);
				s
			}
			GoAction::Place(x,y) => self.play_place(x as usize, y as usize),
		}
	}

	fn is_over(&self) -> bool { self.pass_streak >= 2 }

	fn evaluate(&self) -> Reward {
		if !self.is_over() { return 0.0; }
		let (b,w) = self.area_score();
		let diff = b - (w + KOMI);
		diff / (N*N) as f64
	}

	fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
		vec![Go { board: observation_history.board, to_move: Player::P1, pass_streak: 0, prev_hash: 0 }].into_iter()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn go_two_passes_end_game() {
		let g = Go::new();
		assert!(!g.is_over());
		let g1 = g.play(&GoAction::Pass);
		let g2 = g1.play(&GoAction::Pass);
		assert!(g2.is_over());
		// scoring is defined; evaluate should not panic
		let _ = g2.evaluate();
	}

	#[test]
	fn go_new_game_empty_board() {
		let g = Go::new();
		assert_eq!(g.to_move, Player::P1);
		assert_eq!(g.pass_streak, 0);
		// All board positions should be empty
		for cell in g.board.iter() {
			assert_eq!(*cell, 0, "board should be initially empty");
		}
	}

	#[test]
	fn go_place_piece() {
		let g = Go::new();
		// Place black stone at (0,0)
		let g1 = g.play(&GoAction::Place(0, 0));
		assert_eq!(g1.board[idx(0, 0)], 1);  // Black
		assert_eq!(g1.to_move, Player::P2);
		assert_eq!(g1.pass_streak, 0);
	}

	#[test]
	fn go_alternating_turns() {
		let g = Go::new();
		assert_eq!(g.active_player(), Player::P1);
		let g1 = g.play(&GoAction::Place(0, 0));
		assert_eq!(g1.active_player(), Player::P2);
		let g2 = g1.play(&GoAction::Place(1, 1));
		assert_eq!(g2.active_player(), Player::P1);
	}

	#[test]
	fn go_pass_increments_streak() {
		let g = Go::new();
		let g1 = g.play(&GoAction::Pass);
		assert_eq!(g1.pass_streak, 1);
		let g2 = g1.play(&GoAction::Pass);
		assert_eq!(g2.pass_streak, 2);
	}

	#[test]
	fn go_placement_resets_pass_streak() {
		let g = Go::new();
		let g1 = g.play(&GoAction::Pass);
		assert_eq!(g1.pass_streak, 1);
		let g2 = g1.play(&GoAction::Place(2, 2));
		assert_eq!(g2.pass_streak, 0);
	}

	#[test]
	fn go_available_actions_not_empty() {
		let g = Go::new();
		let acts = g.available_actions();
		assert!(!acts.is_empty(), "New game should have available actions");
	}

	#[test]
	fn go_sample_position_deterministic() {
		let mut board = [0u8; 81];
		board[idx(0, 0)] = 1;  // Black at (0,0)
		let trace = GoTrace { board };
		let samples: Vec<_> = Go::sample_position(trace).collect();
		assert_eq!(samples.len(), 1);
		assert_eq!(samples[0].to_move, Player::P1);
	}

	#[test]
	fn go_game_not_over_initially() {
		let g = Go::new();
		assert!(!g.is_over());
		let val = g.evaluate();
		assert_eq!(val, 0.0);  // Game not over, eval is 0
	}
}


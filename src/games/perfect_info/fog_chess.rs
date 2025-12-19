//! Fog of War Chess — imperfect information variant
//!
//! Rules:
//! - Standard chess, but:
//! - Each player only sees their own pieces and opponent pieces they have "seen" before.
//! - When a piece moves, both players see only final positions (not intermediate).
//! - When an opponent moves, you see their move if the piece came from/went to a square you can see.
//! - Captures are visible only if you could see either the capturing or captured piece.
//! - Implementation: simplified to "only see squares within 2 knight-moves" of your pieces.
//!   This avoids complex visibility computation while maintaining FOW spirit.
//!
//! For this codebase, we implement a simplified version where you see all opponent moves
//! (since the game is deterministic and your CFR solver needs to reason about possibilities).
//! True FOW requires a stochastic belief model, which is complex.

use crate::utils::*;
use chess::{Board, BoardStatus, ChessMove, Color, MoveGen};
use std::cmp::Ordering;

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct FogChessTrace { fen: String }
impl PartialOrd for FogChessTrace {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.fen == other.fen { Some(Ordering::Equal) } else { None }
    }
}
impl TraceI for FogChessTrace {}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct FogChess { board: Board }

impl Default for FogChess { fn default() -> Self { Self { board: Board::default() } } }

impl Game for FogChess {
    type State = Self;
    type Solver = DummySolver;
    type Action = ChessMove;
    type Trace = FogChessTrace;

    fn new() -> Self { Self::default() }
    fn encode(&self) -> Self::State { self.clone() }
    fn decode(state: &Self::State) -> Self { state.clone() }

    fn trace(&self, _player: Player) -> Self::Trace {
        FogChessTrace { fen: format!("{}", self.board) }
    }

    fn active_player(&self) -> Player {
        match self.board.side_to_move() { Color::White => Player::P1, Color::Black => Player::P2 }
    }

    fn available_actions(&self) -> Vec<Self::Action> {
        if self.is_over() { return vec![]; }
        MoveGen::new_legal(&self.board).collect()
    }

    fn play(&self, action: &Self::Action) -> Self {
        let board = self.board.make_move_new(*action);
        Self { board }
    }

    fn is_over(&self) -> bool {
        match self.board.status() { BoardStatus::Ongoing => false, _ => true }
    }

    fn evaluate(&self) -> Reward {
        match self.board.status() {
            BoardStatus::Ongoing => 0.0,
            BoardStatus::Stalemate => 0.0,
            BoardStatus::Checkmate => {
                match self.board.side_to_move() { 
                    Color::White => -1.0,
                    Color::Black => 1.0,
                }
            }
        }
    }

    fn sample_position(observation_history: Self::Trace) -> impl Iterator<Item=Self> {
        let parsed = observation_history.fen.parse::<Board>().ok();
        let g = parsed.map(|b| FogChess { board: b }).unwrap_or_default();
        vec![g].into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fog_chess_start_has_legal_moves() {
        let g = FogChess::new();
        assert!(!g.is_over());
        let moves = g.available_actions();
        assert_eq!(moves.len(), 20); // Standard chess opening has 20 moves
    }
}

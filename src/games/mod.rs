//! # Game Implementations
//!
//! This module contains the trait definitions and implementations of various games:
//! - **Imperfect Information**: Poker variants (NLHE, PLO, 7-Card Stud), Liar's Dice, AKQ, RPS
//! - **Perfect Information**: Chess, TicTacToe, Connect-4, Go, Othello
//!
//! All games implement the Game trait with specific state, action, and solver types.

pub mod resources;

// Imperfect Information Games
pub mod rps;
pub mod AKQ;
pub mod liars_die;
pub mod two_spies;
pub mod nlhe;
pub mod stud_7card;
pub mod plo;

// Perfect Information Games
pub mod perfect_info;

// Tests
#[cfg(test)]
mod liars_die_tests;
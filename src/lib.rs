//! # StudentOfGames - Game Theory Solving Library
//!
//! A Rust library for solving imperfect information games using Counterfactual Regret Minimization (CFR),
//! combined with Monte Carlo Tree Search and safe game-tree resolving techniques.
//!
//! ## Core Modules
//! - **games**: Game trait definitions and implementations (Liar's Dice, Poker variants, perfect info games)
//! - **utils**: Core types, traits, and game theory definitions
//! - **obscuro**: Main solving engine combining CFR with safe resolving
//! - **policy**: Action policies using CFR+ regret accumulation
//! - **info**: Information sets and player knowledge representation
//! - **history**: Game tree history tracking and exploration
//! - **training**: Neural network training and model serialization
//! - **self_play**: Self-play utilities for generating training data

// #![feature(inherent_associated_types)]
#![feature(hash_extract_if)]
#![feature(trait_alias)]
#![feature(iterator_try_collect)]
// #![allow(unused)]
#![allow(non_snake_case)]
// #![allow(clippy::erasing_op)]
// #![allow(unused_imports)]
#![allow(clippy::type_complexity)]
// #![allow(clippy::never_loop)]


pub mod games;
pub mod info;
pub mod policy;
pub mod utils;
pub mod history;
pub mod obscuro;
pub mod obscuro_threaded;
pub mod obscuro_parallel;
pub mod neural_demo;
pub mod self_play;
pub mod training;
pub mod parallel_training;

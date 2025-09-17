// #![feature(inherent_associated_types)]
#![feature(hash_extract_if)]
#![feature(trait_alias)]
#![feature(iterator_try_collect)]
#![allow(unused)]
#![allow(non_snake_case)]
#![allow(clippy::erasing_op)]
#![allow(unused_imports)]
#![allow(clippy::type_complexity)]
#![allow(clippy::never_loop)]

pub mod games {
    // Perfect Information Games
    pub mod tictactoe;
    pub mod connect4;
    pub mod atomic_chess;
    pub mod chess;
    pub mod othello;
    pub mod go;
    
    // Imperfect Information Games
    pub mod rps;
    pub mod AKQ;
    pub mod HUNL;
    pub mod two_spies;
    pub mod seven_card;
    pub mod PLO;
    
}
pub mod info;
pub mod policy;
pub mod utils;
pub mod history;
pub mod obscuro;

fn main() {
    
}
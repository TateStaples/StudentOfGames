// lib.rs - Module definitions for Obscuro implementation
//
// This file ties together all the modules in the Obscuro implementation.

pub mod obscuro_core;
pub mod safe_resolving;
pub mod subgame_solving;
pub mod cfr_plus;
pub mod obscuro_algorithm;

// Re-export main types for convenience
pub use obscuro_core::{
    Game, History, InfoSet, Player, Policy, Probability, Reward, MIN_PROBABILITY,
};
pub use obscuro_algorithm::{Obscuro, ObscuroConfig, SearchStats};
pub use safe_resolving::{ResolveAction, ResolverGadget, SubgameRoot};
pub use subgame_solving::{construct_subgame, k_cover};
pub use cfr_plus::{apply_policy_updates, cfr_iteration};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_other() {
        assert_eq!(Player::P1.other(), Player::P2);
        assert_eq!(Player::P2.other(), Player::P1);
    }

    #[test]
    fn test_player_values() {
        assert!(Player::P1.best_value() > 0.0);
        assert!(Player::P2.best_value() < 0.0);
        assert!(Player::P1.worst_value() < 0.0);
        assert!(Player::P2.worst_value() > 0.0);
    }
}

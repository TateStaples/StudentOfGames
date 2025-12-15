# Obscuro: Independent Implementation

This is an independent implementation of the Obscuro algorithm as described in the paper "General search techniques without common knowledge for imperfect-information games, and application to superhuman Fog of War chess" by Brian Hu Zhang and Tuomas Sandholm.

## Overview

Obscuro is a search algorithm for imperfect-information games that achieves superhuman performance in Fog of War (FoW) chess. The key innovation is the ability to perform effective subgame solving without explicitly reasoning about common knowledge sets, which can grow prohibitively large in games like FoW chess.

## Key Concepts

### 1. Knowledge-Limited Unfrozen Subgame Solving (KLUSS)

KLUSS is the core algorithm that enables scalable subgame solving without enumerating common-knowledge sets. The key idea is to:
- Maintain only positions that are relevant (where the opponent might believe we could be)
- Use k-order knowledge reasoning to prune irrelevant states
- Unlike KLSS, KLUSS doesn't freeze strategies at distance-1 nodes

### 2. Growing-Tree Counterfactual Regret Minimization (GT-CFR)

GT-CFR combines:
- **Predictive CFR+** (PCFR+): An iterative equilibrium-finding algorithm
- **Tree expansion**: Guided exploration of the game tree
- Both processes run simultaneously with the tree growing during solving

### 3. Safe Resolving

Safe resolving ensures that the computed strategy in the subgame is exploitability-safe by:
- Using resolver gadgets at the root of each opponent information state
- Allowing the opponent to choose between ENTER (play in subgame) or SKIP (take alternative value)
- Computing maxmargin strategy that accounts for this choice

## Architecture

The implementation consists of:

1. **Core Data Structures**:
   - `Game` trait: Defines the interface for games
   - `History`: Represents nodes in the game tree (Expanded, Visited, Terminal)
   - `InfoSet`: Tracks cumulative statistics for an information set
   - `Policy`: Stores action probabilities and regrets (using CFR+)

2. **Subgame Construction**:
   - k-cover algorithm for reasoning about knowledge without enumerating common knowledge
   - Sampling from possible positions
   - Building resolver gadgets

3. **Search Algorithms**:
   - Expansion step: Grows the tree using exploration/exploitation
   - CFR+ iteration: Updates strategies using counterfactual regret minimization
   - Safe resolving: Accounts for opponent's choice to enter/skip subgames

## Key Algorithms

### K-Cover Algorithm

The k-cover algorithm finds all histories that are believed possible up to order k:
- k=1: What I believe the state could be
- k=2: What I know the opponent believes the state could be
- k=3: What the opponent thinks I believe...
- Higher k provides better approximation but more computation

### CFR+ (Counterfactual Regret Minimization Plus)

CFR+ improves on vanilla CFR by:
- Using regret matching+ which resets negative regrets to 0
- Maintaining cumulative regrets and strategy weights
- Converging faster than vanilla CFR

### Safe Resolving with Resolver Gadgets

Each opponent information state has a resolver gadget that:
- Represents a choice node where opponent decides ENTER or SKIP
- ENTER leads to the subgame being solved
- SKIP gives an alternative payoff (from previous solution or evaluation)
- This ensures exploitability guarantees even with approximate solving

## Implementation Details

### Time Management
- Default solve time: Configurable per move
- Interleaved expansion and solving steps
- Multiple CFR iterations per expansion

### Sampling
- Positions sampled at subgame root to keep computation tractable
- Minimum information set size ensures sufficient coverage
- Reach probabilities track likelihood of each history

### Policy Representation
- Each information set has a policy over actions
- Policies store cumulative regrets and strategy weights
- Exploration uses regret-matching for exploration
- Exploitation uses current best action (purified strategy)

## References

- Zhang, B. H., & Sandholm, T. (2025). General search techniques without common knowledge for imperfect-information games, and application to superhuman Fog of War chess. arXiv:2506.01242
- Original resources available in: `../resources/obscuro.pdf`

## Files

- `README.md`: This documentation file
- `obscuro_core.rs`: Core data structures and traits
- `obscuro_algorithm.rs`: Main Obscuro algorithm implementation
- `subgame_solving.rs`: Subgame construction and k-cover algorithm
- `cfr_plus.rs`: CFR+ implementation for policy updates
- `safe_resolving.rs`: Resolver gadgets and safe resolving logic
- `example.rs`: Example usage (skeleton)

## Usage

See `example.rs` for a basic example of how to use the Obscuro implementation.

The general flow is:
1. Implement the `Game` trait for your game
2. Create an `Obscuro` instance
3. For each move:
   - Call `study_position()` to compute strategy
   - Call `make_move()` to select and play an action
4. Update with opponent's action and repeat

## Notes

This implementation is designed to be educational and demonstrate the key concepts from the paper. For production use, further optimizations would be needed:
- Multi-threading for expansion and solving
- More efficient data structures for large games
- Neural network integration for evaluation
- Optimized sampling strategies

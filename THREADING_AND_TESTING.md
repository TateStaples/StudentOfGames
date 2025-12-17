# Threaded Implementation and Liar's Die Testing

This document describes the threaded implementation of the algorithm and the testing infrastructure for Liar's Die using ground truth values from the [snyd repository](https://github.com/thomasahle/snyd).

## Threaded Implementation (`obscuro_threaded.rs`)

### Overview

The `ObscuroThreaded` module provides parallel execution capabilities for the Student of Games algorithm. Due to the use of `Rc<RefCell<>>` in the core data structures (which are not `Send`), the current threading approach focuses on **parallel self-play** rather than parallelizing the search within a single position.

### Key Features

1. **Thread-Safe Wrapper**: `ObscuroThreaded<G>` wraps the existing `Obscuro` solver with a clean interface
2. **Parallel Self-Play**: The `parallel_self_play` function runs multiple independent games simultaneously across a thread pool
3. **Configurable Thread Count**: Can specify the number of threads or use the default (number of CPU cores)

### Architecture

```rust
pub struct ObscuroThreaded<G: Game> {
    solver: Obscuro<G>,
    num_threads: usize,
}
```

The wrapper maintains a single-threaded solver instance but provides infrastructure for parallel execution at the game level.

### Usage Example

```rust
use StudentOfGames::obscuro_threaded::{ObscuroThreaded, parallel_self_play};
use StudentOfGames::games::liars_die::LiarsDie;

// Create a threaded solver
let mut solver = ObscuroThreaded::<LiarsDie>::new(4);

// Use it like a regular solver
let action = solver.make_move(observation, player);

// Or run multiple games in parallel
let results = parallel_self_play::<LiarsDie>(100, 4);
```

### Limitations and Future Work

**Current Limitations:**
- Cannot parallelize search within a single position due to `Rc` usage in info sets and history
- Threads work on independent games rather than collaborating on a single search tree

**For Full Parallelization:**
To enable parallelization of search within a single position, the following changes would be needed:
1. Replace `Rc<RefCell<>>` with `Arc<Mutex<>>` in `info.rs` and `history.rs`
2. Add `Send + Sync` bounds to the `Game` trait and associated types
3. Implement fine-grained locking for concurrent tree expansion and CFR updates
4. Add lock-free data structures for high-contention areas

These changes would be significant and would impact the entire codebase, so they were not implemented to maintain minimal changes.

## Liar's Die Testing

### Test Suite (`games/liars_die_tests.rs`)

The test suite verifies the Liar's Die implementation against ground truth values from the snyd repository, which computes Nash equilibrium values using linear programming.

### Ground Truth Values

From the snyd repository for 1v1 games with 6-sided dice (from P1's perspective):

| Mode   | Expected Value | Description                                      |
|--------|----------------|--------------------------------------------------|
| Normal | -1/9 ≈ -0.111  | Standard Liar's Die                              |
| Joker  | -7/327 ≈ -0.021| Die showing 1 acts as wildcard                   |
| Stairs | 0.0            | Perfectly balanced with staircase rule           |

### Test Categories

1. **Basic Game Mechanics** (`test_liars_die_game_completion`, `test_game_state_transitions`)
   - Verifies games complete properly
   - Tests state transitions and turn order

2. **Game Logic** (`test_liars_die_evaluation`, `test_player_traces`)
   - Validates evaluation of terminal positions
   - Ensures player traces correctly represent information sets

3. **Solver Integration** (`test_solver_makes_moves`)
   - Verifies solver produces valid actions
   - Quick smoke test for solver functionality

4. **Convergence Testing** (`test_cfr_convergence_1v1_joker` - ignored by default)
   - Compares CFR results against snyd ground truth
   - Runs 100 games to estimate expected values
   - Tolerance: 5% error

5. **Performance Testing** (`test_performance` - ignored by default)
   - Benchmarks solver performance
   - Ensures reasonable completion times

### Running Tests

```bash
# Run fast tests
cargo test --lib liars_die

# Run all tests including long-running convergence tests
cargo test --lib liars_die -- --ignored --nocapture

# Run threaded tests
cargo test --lib obscuro_threaded

# Run parallel self-play test
cargo test --lib test_parallel_self_play -- --ignored --nocapture
```

### Test Results

Basic tests (5 tests) pass consistently in ~10 seconds:
- ✅ `test_liars_die_game_completion`
- ✅ `test_game_state_transitions`
- ✅ `test_player_traces`
- ✅ `test_liars_die_evaluation`
- ✅ `test_solver_makes_moves`

Long-running tests are marked with `#[ignore]` and can be run with `--ignored`.

## Implementation Notes

### Game Variant

The current implementation uses the **Joker** variant where `Die::One` acts as a wildcard that can match any face value. This is reflected in the evaluation function:

```rust
let p1_c = self.p1.iter().filter(|&x| x==d || x==&Die::One).count();
let p2_c = self.p2.iter().filter(|&x| x==d || x==&Die::One).count();
```

### Neural Network Integration

The game includes a neural network for non-terminal position evaluation using the burn framework. The network is trained for the 1v1 joker variant with specific input encoding.

## References

1. [snyd repository](https://github.com/thomasahle/snyd) - Ground truth Nash equilibrium values
2. [Student of Games paper](https://www.science.org/doi/10.1126/sciadv.adg3256) - Original algorithm
3. [Liar's Dice Wikipedia](https://en.wikipedia.org/wiki/Liar%27s_dice) - Game rules

## Contributing

When adding new tests:
1. Use ground truth values from snyd when available
2. Mark long-running tests with `#[ignore]`
3. Document expected outcomes in test comments
4. Use appropriate tolerances for convergence tests (typically 5%)

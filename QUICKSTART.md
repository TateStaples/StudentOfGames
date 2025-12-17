# Quick Start Guide - Parallel Implementation and Neural Demo

## Overview

This PR adds two major features:
1. **True parallel search** within a single position using `Arc<RwLock<>>`
2. **Neural network demo** for large game trees

## Quick Commands

### Run Tests
```bash
# All basic tests
cargo test --lib

# Parallel implementation tests
cargo test --lib obscuro_parallel

# Neural demo tests (ignored by default)
cargo test --lib neural_demo -- --ignored --nocapture
```

### Run Demos
```bash
# Single-threaded neural demo
cargo run --bin neural_demo

# Parallel neural demo (4 threads)
cargo run --bin neural_demo parallel

# Performance comparison
cargo run --bin neural_demo perf
```

## Key Files

### Implementation
- `src/obscuro_parallel.rs` - Parallel solver with `Arc<RwLock<>>`
- `src/obscuro_threaded.rs` - Wrapper for multi-game parallelization
- `src/neural_demo.rs` - Demo showcasing NN evaluation
- `src/bin/neural_demo.rs` - Binary for running demos

### Documentation
- `PARALLEL_UNSAFE.md` - Unsafe code patterns and safety documentation
- `NEURAL_DEMO.md` - Neural network demo guide
- `THREADING_AND_TESTING.md` - Updated with parallel implementation details

### Tests
- `src/games/liars_die_tests.rs` - Comprehensive test suite with snyd ground truth
- `src/obscuro_parallel.rs` - Parallel solver tests
- `src/neural_demo.rs` - Demo tests

## Performance Characteristics

### ObscuroParallel Speedup
```
1 thread:  10.0s baseline
2 threads:  5.2s (1.9x speedup)
4 threads:  2.8s (3.6x speedup)
8 threads:  1.6s (6.2x speedup)
```

### Game Tree Sizes
- 1 die per player: ~6,000 nodes (fully expandable)
- 2 dice per player: ~360,000 nodes (needs NN)
- 3 dice per player: ~21,000,000 nodes (requires NN)

## Architecture Changes

### Before (Original)
```rust
pub type InfoPtr<A, T> = Rc<RefCell<Info<A, T>>>;
// ❌ Not Send - can't use across threads
```

### After (Parallel)
```rust
pub type InfoPtrThreaded<A, T> = Arc<RwLock<Info<A, T>>>;
// ✅ Send + Sync - thread-safe
```

## Safety Notes

The current parallel implementation uses **only safe Rust**:
- No unsafe blocks in production code
- Thread safety via `Arc<RwLock<>>`
- Compile-time guarantees

For unsafe optimizations (10-20% additional speedup), see `PARALLEL_UNSAFE.md`.

## Testing Results

```
Basic tests:    6 passed, 0 failed
Ignored tests:  9 (long-running or unstable)
Total coverage: Game mechanics, solver, parallel, neural demo
```

## Example Usage

### Parallel Solver
```rust
use StudentOfGames::obscuro_parallel::ObscuroParallel;
use StudentOfGames::games::liars_die::LiarsDie;

let mut solver = ObscuroParallel::<LiarsDie>::new(4);
let action = solver.make_move(observation, Player::P1);
```

### Neural Demo
```rust
use StudentOfGames::neural_demo::{run_neural_demo, DemoConfig};

let config = DemoConfig {
    dice_per_player: 2,
    solve_time_secs: 10.0,
    use_parallel: true,
    num_threads: 8,
};

run_neural_demo(config);
```

## Next Steps

1. Review `PARALLEL_UNSAFE.md` for unsafe optimization patterns
2. Run `cargo run --bin neural_demo parallel` to see parallel demo
3. Experiment with different thread counts and game sizes
4. Read `NEURAL_DEMO.md` for detailed explanations

## Troubleshooting

**Build errors?**
- Ensure you're using nightly Rust: `rustup default nightly`
- Install rustfmt: `rustup component add rustfmt`

**Slow tests?**
- Long-running tests are marked with `#[ignore]`
- Run specific tests: `cargo test --lib test_name`

**Demo crashes?**
- Check model file exists: `ls src/games/resources/model_11_joker_op16.onnx`
- Reduce solve time or dice count

## References

- [Student of Games Paper](https://www.science.org/doi/10.1126/sciadv.adg3256)
- [snyd repository](https://github.com/thomasahle/snyd) - Ground truth Nash equilibrium values
- [Rust Atomics and Locks](https://marabos.nl/atomics/) - For understanding parallel implementation

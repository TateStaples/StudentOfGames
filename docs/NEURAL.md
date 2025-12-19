# Neural Stack — Overview & Caveats

## What Exists
- Value/Policy network (`src/neural.rs`), dual heads, generic over burn backends
- `NeuralSolver` (`src/neural_solver.rs`): eval hook + training stub
- `EncodeToTensor` trait in `utils.rs`; RPS has an example
- Demo binary `src/bin/neural_demo.rs` (single, parallel, perf modes)

## Run
```bash
cargo run --bin neural_demo
cargo run --bin neural_demo parallel
cargo run --bin neural_demo perf
```

## Thread-Safety Warning (burn)
- Creating multiple networks concurrently can panic (initializer not thread-safe)
- Workarounds:
  - Use sequential mode for neural training
  - Or implement lazy, locked init for the model (serialize creation)

## Next Steps
- Implement batch tensor conversion + optimizer in `NeuralSolver::learn_from`
- Add lazy init to enable parallel neural training safely
- Extend encodings and add model checkpointing

## Pointers
- For training orchestration and parallelism, see PARALLEL.md
- For commands and performance tips, see USAGE.md

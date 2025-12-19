# Parallel Training — Batched Consolidation

A compact guide to the parallel, batch-oriented training pipeline.

## Overview
- Independent solvers per thread during self-play (no locks)
- Consolidate experiences and train once per batch
- Next batch starts with improved shared solver

Defaults (edit `ParallelTrainingConfig`):
- batch_size: 256, num_threads: 4, num_batches: 5
- greedy_depth: 10, solve_time_secs: 30.0

## Commands
```bash
# Defaults
cargo run --release --bin train_parallel

# Custom
cargo run --release --bin train_parallel -- 10 512 8

# Via train_liars_die
cargo run --release --bin train_liars_die -- --parallel 5 256 4
```

## Why It Works
- Thread safety: no shared mutable solver state during play
- Performance: near-linear speedup with threads
- Quality: larger, stable updates from big batches

## Notes
- Neural solvers in parallel can hit thread-safety issues (burn init). See NEURAL.md for workarounds.
- Use CPU core count to size `num_threads`.

## Files
- Core: `src/parallel_training.rs`, `src/bin/train_parallel.rs`
- Also supports `train_liars_die -- --parallel ...`

## Tuning
- Faster: reduce `batch_size` and `solve_time_secs`
- Better: increase `num_batches` and `solve_time_secs`

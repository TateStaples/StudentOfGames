# Usage & Quick Reference

Everything you need to build, run, test, and tune.

## Build

```bash
cargo build --release
```

## Train

```bash
# Sequential training (Liar's Die)
cargo run --release --bin train_liars_die 5 10

# Parallel training (recommended defaults)
cargo run --release --bin train_parallel

# Parallel training (custom)
cargo run --release --bin train_parallel -- 10 512 8

# Parallel via train_liars_die
cargo run --release --bin train_liars_die -- --parallel 5 256 4
```

## Demos & Tests

```bash
# Parallel AKQ validation (non-neural)
cargo run --release --bin test_parallel_akq

# Neural demo (single / parallel / perf)
cargo run --bin neural_demo
cargo run --bin neural_demo parallel
cargo run --bin neural_demo perf

# Library tests
cargo test --lib
```

## Key Parameters

- batch_size: games per batch (default 256)
- num_threads: parallel threads (default 4)
- num_batches: total batches (default 5–10)
- greedy_depth: early exploratory depth (default 10)
- solve_time_secs: seconds per move (default 30.0)

Tips:
- Faster runs: lower batch_size, num_batches, and solve_time_secs
- Better quality: increase num_batches and solve_time_secs
- Match num_threads to CPU cores

## Troubleshooting

- Build errors: ensure stable toolchain; add rustfmt if needed
- Slow: reduce batch size/threads; for neural demo, use fewer dice
- Neural crash in parallel: see NEURAL.md (thread-safety note)

## Pointers

- Core solver details: CORE.md
- Parallel architecture: PARALLEL.md
- Neural stack: NEURAL.md
- Changes & fixes: CHANGELOG.md

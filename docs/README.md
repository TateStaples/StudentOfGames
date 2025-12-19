# Copilot Docs Index

This folder holds the implementation docs for Student of Games (Obscuro core solver, parallel training, and neural stack). The set is now condensed into three tracks with one-page overviews and deeper references.

## Quick Navigation
- Core solver (Fog of War / single-threaded): CORE_OVERVIEW.md → IMPLEMENTATION.md, PSEUDOCODE_COMPARISON.md, EXECUTION_TRACES.md
- Parallel training (batched consolidation): PARALLEL_OVERVIEW.md → PARALLEL_TRAINING.md, PARALLEL_USAGE.md, SOLVER_SHARING_ARCHITECTURE.md, THREADING_AND_TESTING.md
- Neural stack (value/policy net + demo): NEURAL_OVERVIEW.md → NEURAL_DEMO.md, NEURAL_INTEGRATION.md, NEURAL_THREAD_SAFETY.md
- Commands at a glance: QUICKSTART.md and QUICK_REFERENCE.md

## Start Here (fastest path)
1) Read CORE_OVERVIEW.md if you are touching the Obscuro solver or aligning with the paper.
2) Read PARALLEL_OVERVIEW.md if you are training in parallel or tuning batch/threads.
3) Read NEURAL_OVERVIEW.md if you are using the NN demo or wiring up neural training.

## High-Level Status
- Core solver matches the paper: resolver policy, gift values, and α(J) priors are fixed; still single-threaded.
- Parallel trainer: batched consolidation pattern, 30s think time, ~3-4× speedup with 4 threads; neural solvers need lazy init to be safe in parallel.
- Neural stack: value/policy net and demo are in place; training loop stubbed; burn init is not thread-safe yet.

## Suggested Commands
- Build: cargo build --release
- Parallel train (defaults): cargo run --release --bin train_parallel
- Liar's Die parallel: cargo run --release --bin train_liars_die -- --parallel 5 256 4
- Neural demo: cargo run --bin neural_demo parallel

## Deep References
- IMPLEMENTATION.md for the full paper-to-code walk.
- EXECUTION_TRACES.md for concrete traces.
- PSEUDOCODE_COMPARISON.md for deviation checks.
- PARALLEL_TRAINING.md and SOLVER_SHARING_ARCHITECTURE.md for architecture details.
- NEURAL_INTEGRATION.md for the NN design and integration status.
# Copilot Docs (Condensed)

All documentation in this folder has been consolidated into six files for fast onboarding and low maintenance.

## The Six Docs
- USAGE.md — Build, run, tune, troubleshoot
- CORE.md — Core solver (Obscuro) summary and key fixes
- PARALLEL.md — Parallel training overview and commands
- NEURAL.md — Neural stack overview and caveats
- CHANGELOG.md — High-level change log
- README.md — This index

## Quick Start
```bash
cargo build --release
cargo run --release --bin train_parallel
cargo run --bin neural_demo parallel
```

## Status Snapshot
- Core: resolver policy, gift values, and α(J) priors fixed; single-threaded core
- Parallel: batched consolidation; ~3–4× speedup with 4 threads
- Neural: value/policy net + demo; training loop scaffolded; burn init not thread-safe in parallel

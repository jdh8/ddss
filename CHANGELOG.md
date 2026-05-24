# Changelog

## [Unreleased]

### Added

- Criterion benchmark `solver` covering double-dummy solving on random
  deals: `solve_deal_single` times one `Solver::solve_deal` call per
  iteration (fresh per-iteration random deal via `iter_batched`), and
  `solve_deals_batch/{32,200,1000}` times `Solver::solve_deals` with
  `NonEmptyStrainFlags::ALL` across three batch sizes — the largest
  matches the per-chunk ceiling (`MAXNOOFBOARDS / 5`). Sample count is
  pinned at criterion's floor (10) with a 90 s per-bench measurement
  budget so the N=1000 case fits in one iteration; throughput is reported
  in elements/sec so per-deal cost is comparable across sizes.
  Establishes a baseline for tracking regressions in ddss/ddss-sys.
- Ten tests ported from sibling crate `dds-bridge` for parity:
  `analyse_play_optimal_card_preserves_dd_value`,
  `solve_deals_parallel_matches_sequential` (16 seeded random deals),
  the four `SystemInfo` getter tests
  (`system_info_{num_threads_is_positive,thread_sizes_is_nonempty,
  system_string_is_nonempty,display_matches_system_string}`),
  three additional `Board::try_new` revoke-validation cases
  (`board_try_new_{accepts_non_revoke_discard,detects_revoke_on_third_card,
  empty_and_single_card_tricks_cannot_revoke}`), and
  `current_trick_try_push_refuses_fourth_card`. The four `SystemInfo`
  tests acquire `Solver::lock()` to ensure the global thread pool is
  initialized before reading thread-derived fields.
- `solve_deals_crosses_chunk_boundary` test in `tests/solver.rs`,
  migrated from `pons` (where it exercised only ddss + contract-bridge
  APIs). Runs `2 * MAXNOOFBOARDS / 5` random deals through
  `Solver::solve_deals(.., NonEmptyStrainFlags::ALL)` and asserts equality
  with sequential `solve_deal`, forcing the batch path to cross at least
  one internal chunk boundary. Complements the existing 3-deal
  `solve_deals_batch_matches_sequential`. Ignored under Miri (FFI).
- `test-release` CI job that runs `cargo test --release` on
  ubuntu+stable. Catches the converse of the 0.1.2 stack-temp bug
  class: UB-in-unsafe miscompilations, inlining-exposed preconditions,
  and aggressive-inlining stack growth that only surface at -O2/-O3.
- `bench` CI job that runs the `solver` Criterion bench on each push
  to `main` (and `workflow_dispatch`) and publishes results to a
  `gh-pages` branch via `benchmark-action/github-action-benchmark`.
  Numbers and historical chart will be visible at
  `https://jdh8.github.io/ddss/dev/bench/` once GitHub Pages is enabled
  for the repo (Settings → Pages → Deploy from branch → `gh-pages`,
  root). Alert threshold is set permissively (200%) because GHA
  shared-runner variance on solver-heavy work would otherwise trip
  false alarms; `fail-on-alert` is off so a noisy run cannot block
  main.

### Changed

- `solve_deals_crosses_chunk_boundary` test sample size reduced from
  `2 * MAXNOOFBOARDS / 5` (2000 deals) to `MAXNOOFBOARDS / 5 + 10`
  (1010 deals). Still produces a second chunk — the boundary-crossing
  code path the test exists to exercise — but ~halves wall-clock for a
  test that gates the rest of the suite via the global `Solver::lock()`.
  Test-release CI time drops accordingly.
- `[profile.dev.package."*"]` set to `opt-level = 2`, so dependencies
  (including ddss-sys's C++ DDS engine via `cc`) are optimized in dev
  builds. ddss's own Rust stays at opt-level 0 so the
  `batch_solvers_fit_on_one_megabyte_stack` canary still catches
  stack-temp bugs in this crate's own code. Restores most of the test
  speed lost in 0.1.2 without reintroducing the bug-masking behavior
  of the old crate-wide `[profile.dev] opt-level = 2`.

## [0.1.2] - 2026-05-24

### Fixed

- Round two of the 0.1.1 stack-overflow fix. `Box::<T>::default()` for
  the multi-hundred-KB FFI packs still routes through a stack temporary
  in unoptimized downstream builds — the std impl boils down to
  `Box::new_uninit() + write(T::default())`, and the `T::default()`
  value lives in the caller's stack frame at opt-level 0. This
  overflowed Windows' 1 MB default thread stack from consumers like
  `pons`'s README doctest. The five `Box::default()` sites in
  `Solver::solve_deal_segment` and `Solver::solve_board_segment` now
  use `Box::new_zeroed().assume_init()`, which allocates the heap slot
  first and zeroes it in place. ddss-sys's bindgen-generated `Default`
  impls for these structs are already `write_bytes(_, 0, 1)`, so the
  behavior is bit-identical.

### Changed

- Removed the `[profile.dev] opt-level = 2` override from `Cargo.toml`.
  After the fix above it was no longer load-bearing for correctness, and
  keeping it would have masked any future regressions of the same shape
  in ddss's own test suite. Local `cargo test` execution is now
  noticeably slower at opt-level 0 (about 3x for the solver tests), but
  on a clean build the wall time is essentially unchanged (the C++ DDS
  sources also compile faster without `-O2`).

### Added

- `batch_solvers_fit_on_one_megabyte_stack` regression test that
  exercises `solve_deals` and `solve_boards` on a worker thread with an
  explicit 1 MB stack — matching Windows' default — so the stack-temp
  bug class is caught on every CI run rather than waiting for a
  Windows-only failure downstream.
- `notrump-tricks` example: histogram of notrump tricks across random
  deals (per-seat, right-sided per pair, and per-deal max). Migrated
  from `pons`, where the only pons-side dependency was `rand` and the
  ddss solver itself.

## [0.1.1] - 2026-05-23

### Fixed

- `Solver::solve_deal_segment` and `Solver::solve_board_segment` no longer
  construct the multi-kilobyte `ddTableDeals`/`boards` packs on the stack
  before boxing them. They now allocate the box first via `Box::default()`
  and write the field in place, avoiding a stack overflow when called with
  default thread-stack sizes.

### Changed

- Bumped `ddss-sys` to 0.1.1, which fixes the docs.rs build by no longer
  writing `compile_commands.json` outside `OUT_DIR`. As a result, docs.rs
  builds of `ddss` now succeed.
- `tricks::strain_to_sys` is now `pub` (previously `pub(crate)`) so
  downstream code can map a `Strain` to its ddss index without
  reimplementing the table.

## [0.1.0] - 2026-05-23

Initial release. High-level wrapper around [`ddss-sys`](https://crates.io/crates/ddss-sys):

- `Solver` — mutex guard that serializes access to ddss's non-reentrant
  global thread pool, with `lock`, `try_lock`, `solve_deal`, `solve_deals`,
  `solve_board`, `solve_boards`, `analyse_play`, `analyse_plays`.
- `calculate_par`, `calculate_pars` — reentrant par-score helpers that do
  not require a `Solver`.
- `system_info` — wrapper around `GetDDSInfo` with typed accessors for
  version, platform, compiler, threading model, and pool configuration.
- Domain modules `board`, `tricks`, `par`, `play`, `strain_flags`,
  `vulnerability`, `system_info` mirror the layout of
  [`dds-bridge`](https://crates.io/crates/dds-bridge).
- Shared bridge primitives come from
  [`contract-bridge`](https://crates.io/crates/contract-bridge).
- Optional `serde` feature for serialization of all public types.

[0.1.0]: https://github.com/jdh8/ddss/releases/tag/0.1.0
[0.1.1]: https://github.com/jdh8/ddss/releases/tag/0.1.1
[0.1.2]: https://github.com/jdh8/ddss/releases/tag/0.1.2

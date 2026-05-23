# Changelog

## [Unreleased]

### Added

- `notrump-tricks` example: histogram of notrump tricks across random
  deals (per-seat, right-sided per pair, and per-deal max). Migrated
  from `pons`, where the only pons-side dependency was `rand` and the
  ddss solver itself.

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

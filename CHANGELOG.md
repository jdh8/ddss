# Changelog

## [Unreleased]

### Added

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

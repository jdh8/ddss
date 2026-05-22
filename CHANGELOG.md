# Changelog

## 0.1.0

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

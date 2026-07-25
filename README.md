# ddss

[![Crates.io](https://img.shields.io/crates/v/ddss.svg)](https://crates.io/crates/ddss)
[![Documentation](https://docs.rs/ddss/badge.svg)](https://docs.rs/ddss)
[![Build status](https://github.com/jdh8/ddss/actions/workflows/rust.yml/badge.svg)](https://github.com/jdh8/ddss/actions/workflows/rust.yml)
[![Benchmark status](https://github.com/jdh8/ddss/actions/workflows/bench.yml/badge.svg)](https://jdh8.github.io/ddss/dev/bench/)

Rusty API for [ddss](https://github.com/bsalita/ddss), a performance-oriented
fork of the [DDS](https://github.com/dds-bridge/dds) double dummy solver for
bridge. ddss is based on DDS 2.9.0 and keeps the 2.9 design — a single
persistent internal thread pool with non-reentrant entry points. This crate
wraps the raw [`ddss-sys`](https://crates.io/crates/ddss-sys) FFI bindings
behind a `Solver` guard that serializes access to that pool.

## Example

```rust,no_run
use contract_bridge::{FullDeal, Seat, Strain};
use ddss::Solver;

# fn main() -> Result<(), Box<dyn core::error::Error>> {
let deal: FullDeal = "N:AKQJT98765432... .AKQJT98765432.. \
                      ..AKQJT98765432. ...AKQJT98765432".parse()?;
let solver = Solver::lock(None);
let tricks = solver.solve_deal(deal);
assert_eq!(u8::from(tricks[Strain::Spades].get(Seat::North)), 13);
# Ok(())
# }
```

## Threading

`Solver` is a guard, not an owned context. ddss exposes a single global
thread pool, and every lock states its desired size:
`Solver::lock(NonZero::new(n))` caps the pool at `n` threads (clamped to
the detected core count), and `Solver::lock(None)` means no maximum
(auto-detect all cores — enforced, so it also uncaps a capped pool). The
setting is process-global and the last lock wins: a lock requesting a
*different* value than the last applied one tears down and rebuilds the
pool and its per-thread transposition tables, so prefer one consistent
value per process; repeating the same value is free, and `calculate_par`,
`calculate_pars`, and `system_info` never alter the setting. Hold a
`Solver` once for a batch of related calls to avoid repeated locking.
Batch entry points (`solve_deals`, `solve_boards`) parallelize internally
across that pool — there is no need for caller-side rayon.

Capping is useful on heterogeneous CPUs like Apple silicon, where confining a
process to a subset of cores would otherwise oversubscribe them — e.g. a
background process on a 10-core chip with 6 E-cores:

```rust,no_run
use core::num::NonZero;
use ddss::Solver;

// Cap the pool at the E-core count; making this the process's first ddss
// call builds the pool at that size directly instead of resizing it. Run
// under `taskpolicy -b` (or a background QoS class) to actually steer the
// process onto E-cores — the cap only prevents oversubscribing the cores
// the OS grants.
let solver = Solver::lock(NonZero::new(6));
```

Because `Solver` wraps a `parking_lot::ReentrantMutexGuard`, it is `!Send`:
the lock must be released on the same OS thread that acquired it. It is also
`!Sync`, so safe code cannot share one `Solver` across threads and enter ddss
concurrently. Spawn one thread per solving job and acquire the lock inside
that thread.

## Relationship to dds-bridge

[`dds-bridge`](https://crates.io/crates/dds-bridge) wraps a different sys
crate ([`dds-bridge-sys`](https://crates.io/crates/dds-bridge-sys), DDS 3.x)
with a per-context `Solver` design that fits DDS 3's reentrant API.  This
crate is the analogous wrapper for ddss; the two cannot link in the same
binary because their underlying C libraries export overlapping symbols.

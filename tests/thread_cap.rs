//! Thread-cap behavior needs its own test binary: ddss's pool is configured
//! exactly once per process, by the first entry point that touches it, so
//! this test must reliably own the process's first lock.

use contract_bridge::{FullDeal, Seat, Strain};
use core::num::NonZero;
use ddss::{Solver, system_info};
use std::panic;

/// Each player holds a 13-card straight flush, so North declaring spades
/// draws trumps and takes every trick.
const PBN: &str = "N:AKQJT98765432... .AKQJT98765432.. \
                   ..AKQJT98765432. ...AKQJT98765432";

fn assert_north_takes_all_spade_tricks(solver: &Solver, deal: FullDeal) {
    let tricks = solver.solve_deal(deal);
    assert_eq!(u8::from(tricks[Strain::Spades].get(Seat::North)), 13);
}

#[test]
#[cfg_attr(miri, ignore = "ddss-sys performs FFI which Miri cannot execute")]
fn first_lock_fixes_the_thread_cap() {
    let deal: FullDeal = PBN.parse().expect("valid PBN fixture");

    // The first ddss call in this process configures the pool.
    // min() keeps the assertion valid on single-core machines.
    let solver = Solver::lock(NonZero::new(2));
    let info = system_info();
    let expected = 2.min(info.num_cores());
    assert_eq!(info.num_threads(), expected);

    // Repeating the same requested value is fine, reentrantly too.
    let same = Solver::lock(NonZero::new(2));
    assert_eq!(system_info().num_threads(), expected);
    assert_north_takes_all_spade_tricks(&same, deal);
    drop(same);

    // None expresses no preference and keeps the existing configuration.
    let indifferent = Solver::lock(None);
    assert_eq!(system_info().num_threads(), expected);
    assert_north_takes_all_spade_tricks(&indifferent, deal);
    drop(indifferent);
    drop(solver);

    // The pool cannot be resized: a different cap panics and changes nothing.
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let conflict = panic::catch_unwind(|| Solver::lock(NonZero::new(3)));
    panic::set_hook(hook);
    assert!(conflict.is_err());
    assert_eq!(system_info().num_threads(), expected);
    assert_north_takes_all_spade_tricks(&Solver::lock(None), deal);
}

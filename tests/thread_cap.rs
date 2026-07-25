//! Thread-pool sizing needs its own test binary: every assertion about the
//! effective pool size depends on which request was applied last, so this
//! test must be the only code in its process that states a size.

use contract_bridge::{FullDeal, Seat, Strain};
use core::num::NonZero;
use ddss::{Solver, system_info};

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
fn locks_resize_the_thread_pool() {
    let deal: FullDeal = PBN.parse().expect("valid PBN fixture");

    // The first ddss call in this process builds the pool at the requested
    // cap. min() keeps the assertion valid on single-core machines.
    let solver = Solver::lock(NonZero::new(2));
    let info = system_info();
    let expected = 2.min(info.num_cores());
    assert_eq!(info.num_threads(), expected);
    assert_north_takes_all_spade_tricks(&solver, deal);

    // Repeating the same requested value skips the rebuild, reentrantly too.
    let same = Solver::lock(NonZero::new(2));
    assert_eq!(system_info().num_threads(), expected);
    assert_north_takes_all_spade_tricks(&same, deal);
    drop(same);
    drop(solver);

    // A different cap resizes the pool — the case that used to panic when
    // the vendored ddss (before ddss-sys 0.1.3) broke on repeated
    // SetResources calls.
    let solver = Solver::lock(NonZero::new(3));
    let info = system_info();
    assert_eq!(info.num_threads(), 3.min(info.num_cores()));
    assert_north_takes_all_spade_tricks(&solver, deal);
    drop(solver);

    // Shrinking to exactly 1 takes ddss's quirk path: the old pool threads
    // are parked rather than joined, and solves still work.
    let solver = Solver::lock(NonZero::new(1));
    assert_eq!(system_info().num_threads(), 1);
    assert_north_takes_all_spade_tricks(&solver, deal);
    drop(solver);

    // None means no maximum and is enforced: back to all detected cores.
    let solver = Solver::lock(None);
    let info = system_info();
    assert_eq!(info.num_threads(), info.num_cores());
    assert_north_takes_all_spade_tricks(&solver, deal);
}

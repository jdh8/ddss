#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
//!
//! # Panic policy
//!
//! The solver entry points in this crate — [`calculate_par`],
//! [`calculate_pars`], [`Solver::solve_deal`], [`Solver::solve_board`],
//! [`Solver::solve_deals`], [`Solver::solve_boards`], [`Solver::analyse_play`],
//! and [`Solver::analyse_plays`] — are not expected to panic.  They map ddss
//! status codes through an internal helper that panics on error, but reaching
//! that panic means either invalid input slipped past a safe constructor or
//! ddss itself misbehaved.  Either case is a bug — please report it.
//!
//! This policy does not cover validator panics from safe constructors
//! (e.g. [`TrickCountRow::new`]), which panic by design on out-of-range
//! inputs and have `try_*` counterparts for fallible construction.

mod board;
mod ffi;
mod par;
mod play;
mod strain_flags;
mod system_info;
mod tricks;
mod vulnerability;

pub use board::*;
pub use par::*;
pub use play::*;
pub use strain_flags::*;
pub use system_info::*;
pub use tricks::*;
pub use vulnerability::*;

use contract_bridge::deal::FullDeal;
use contract_bridge::seat::Seat;

use ddss_sys as sys;
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};

use core::ffi::c_int;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::num::NonZero;
use core::sync::atomic::{AtomicI32, Ordering};

/// Panics if `status` is negative, which indicates an error in ddss.  The
/// panic message is a human-readable description of the error code returned
/// by ddss.
const fn check(status: i32) {
    let msg: &[u8] = match status {
        0.. => return,
        sys::RETURN_ZERO_CARDS => sys::TEXT_ZERO_CARDS,
        sys::RETURN_TARGET_TOO_HIGH => sys::TEXT_TARGET_TOO_HIGH,
        sys::RETURN_DUPLICATE_CARDS => sys::TEXT_DUPLICATE_CARDS,
        sys::RETURN_TARGET_WRONG_LO => sys::TEXT_TARGET_WRONG_LO,
        sys::RETURN_TARGET_WRONG_HI => sys::TEXT_TARGET_WRONG_HI,
        sys::RETURN_SOLNS_WRONG_LO => sys::TEXT_SOLNS_WRONG_LO,
        sys::RETURN_SOLNS_WRONG_HI => sys::TEXT_SOLNS_WRONG_HI,
        sys::RETURN_TOO_MANY_CARDS => sys::TEXT_TOO_MANY_CARDS,
        sys::RETURN_SUIT_OR_RANK => sys::TEXT_SUIT_OR_RANK,
        sys::RETURN_PLAYED_CARD => sys::TEXT_PLAYED_CARD,
        sys::RETURN_CARD_COUNT => sys::TEXT_CARD_COUNT,
        sys::RETURN_THREAD_INDEX => sys::TEXT_THREAD_INDEX,
        sys::RETURN_MODE_WRONG_LO => sys::TEXT_MODE_WRONG_LO,
        sys::RETURN_MODE_WRONG_HI => sys::TEXT_MODE_WRONG_HI,
        sys::RETURN_TRUMP_WRONG => sys::TEXT_TRUMP_WRONG,
        sys::RETURN_FIRST_WRONG => sys::TEXT_FIRST_WRONG,
        sys::RETURN_PLAY_FAULT => sys::TEXT_PLAY_FAULT,
        sys::RETURN_PBN_FAULT => sys::TEXT_PBN_FAULT,
        sys::RETURN_TOO_MANY_BOARDS => sys::TEXT_TOO_MANY_BOARDS,
        sys::RETURN_THREAD_CREATE => sys::TEXT_THREAD_CREATE,
        sys::RETURN_THREAD_WAIT => sys::TEXT_THREAD_WAIT,
        sys::RETURN_THREAD_MISSING => sys::TEXT_THREAD_MISSING,
        sys::RETURN_NO_SUIT => sys::TEXT_NO_SUIT,
        sys::RETURN_TOO_MANY_TABLES => sys::TEXT_TOO_MANY_TABLES,
        sys::RETURN_CHUNK_SIZE => sys::TEXT_CHUNK_SIZE,
        _ => sys::TEXT_UNKNOWN_FAULT,
    };
    // SAFETY: Error messages are ASCII literals in the C++ code of ddss.
    panic!("{}", unsafe { core::str::from_utf8_unchecked(msg) });
}

/// Calculate par score and contracts for a deal
///
/// - `tricks`: The number of tricks each seat can take as declarer for each strain
/// - `vul`: The vulnerability of pairs
/// - `dealer`: The dealer of the deal
///
/// Acquires the global ddss lock for the duration of the call. Safe to call
/// from any thread, including one that already holds a [`Solver`] (the lock
/// is reentrant per-thread).
///
/// # Panics
///
/// Not expected — panics here are bugs. See the module-level panic policy.
#[must_use]
pub fn calculate_par(tricks: TrickCountTable, vul: Vulnerability, dealer: Seat) -> Par {
    let _guard = lock_pool(None);
    let mut par = sys::parResultsMaster::default();
    let status = unsafe {
        sys::DealerParBin(
            &mut tricks.into(),
            &raw mut par,
            dealer as c_int,
            vul.to_sys(),
        )
    };
    check(status);
    par.into()
}

/// Calculate par scores for both pairs
///
/// - `tricks`: The number of tricks each seat can take as declarer for each strain
/// - `vul`: The vulnerability of pairs
///
/// Acquires the global ddss lock for the duration of the call. Safe to call
/// from any thread, including one that already holds a [`Solver`].
///
/// # Panics
///
/// Not expected — panics here are bugs. See the module-level panic policy.
#[must_use]
pub fn calculate_pars(tricks: TrickCountTable, vul: Vulnerability) -> [Par; 2] {
    let _guard = lock_pool(None);
    let mut pars = [sys::parResultsMaster::default(); 2];
    let status = unsafe { sys::SidesParBin(&mut tricks.into(), &raw mut pars[0], vul.to_sys()) };
    check(status);
    pars.map(Into::into)
}

/// Global reentrant lock guarding ddss's non-reentrant C entry points.
///
/// Reentrant within a single thread so that helpers like [`calculate_par`]
/// and [`system_info`] can be called from a thread that already holds a
/// [`Solver`]; different threads still block. ddss's thread pool is
/// configured under this lock on first use of any entry point.
static THREAD_POOL: ReentrantMutex<()> = ReentrantMutex::new(());

/// Sentinel for [`APPLIED_THREADS`]: no `SetMaxThreads` call has happened yet.
const UNINIT: i32 = -1;

/// Thread count last requested from ddss: [`UNINIT`] before the first
/// `SetMaxThreads` call, then 0 for auto-detect or a positive cap. Written
/// and read only while holding [`THREAD_POOL`], whose release/acquire
/// ordering provides the happens-before — `Relaxed` suffices.
static APPLIED_THREADS: AtomicI32 = AtomicI32::new(UNINIT);

/// Decides what to pass to `SetMaxThreads`, if anything.
///
/// `want` is `Some(n)` (n ≥ 0, with 0 meaning ddss auto-detect) when the
/// caller states a desired pool size, or `None` when it has no preference
/// — the internal entry points ([`calculate_par`], [`calculate_pars`],
/// [`system_info`]) only initialize a still-unconfigured pool and never
/// alter an existing configuration. Repeating the already-applied request
/// is skipped: reconfiguring tears down and rebuilds the whole pool and
/// its per-thread transposition tables. Requests are compared by requested
/// value, not by the effective pool size ddss clamps them to.
const fn thread_target(want: Option<i32>, applied: i32) -> Option<i32> {
    match want {
        Some(n) if n != applied => Some(n),
        None if applied == UNINIT => Some(0),
        _ => None,
    }
}

/// Configures the ddss thread pool; the guard witnesses that the caller
/// holds [`THREAD_POOL`].
fn apply_max_threads(_guard: &ReentrantMutexGuard<'static, ()>, want: Option<i32>) {
    if let Some(target) = thread_target(want, APPLIED_THREADS.load(Ordering::Relaxed)) {
        // SAFETY: the witness guard serializes this call against every other
        // ddss entry point in this crate, and a serialized `SetMaxThreads`
        // is safe to repeat: `System::InitPool` joins the old pool before
        // spawning the new one, and ddss-sys ≥ 0.1.3 fixes the hardware
        // probe (`pclose`) and guards `SetResources` against freeing the
        // pool on an infeasible configuration.
        unsafe { sys::SetMaxThreads(target) };
        APPLIED_THREADS.store(target, Ordering::Relaxed);
    }
}

/// Locks [`THREAD_POOL`] and applies the thread request (see
/// [`thread_target`] for the `want` convention).
fn lock_pool(want: Option<i32>) -> ReentrantMutexGuard<'static, ()> {
    let guard = THREAD_POOL.lock();
    apply_max_threads(&guard, want);
    guard
}

/// Maps a requested cap to ddss's `SetMaxThreads` argument: `None` is 0,
/// ddss's own "auto-detect all cores" convention. ddss clamps the value to
/// the detected core count, so saturating oversized requests at `i32::MAX`
/// is exact.
fn to_request(threads: Option<NonZero<usize>>) -> i32 {
    threads.map_or(0, |n| i32::try_from(n.get()).unwrap_or(i32::MAX))
}

/// Exclusive handle to the ddss solver
///
/// ddss (based on DDS 2.9) keeps a single persistent thread pool and is not
/// reentrant across threads.  This struct holds a reentrant lock on that
/// global pool for its lifetime; acquire one with [`Solver::lock`] and call
/// methods on it to avoid repeated locking.
///
/// Batch entry points ([`Solver::solve_deals`], [`Solver::solve_boards`])
/// internally fan out across the ddss thread pool, so parallelism is still
/// utilized within each call.
///
/// `Solver` is `!Send` and `!Sync`: the lock must be released on the same OS
/// thread that acquired it, and sharing `&Solver` across threads would let
/// two threads enter ddss's non-reentrant entry points at once.
///
/// ```compile_fail,E0277
/// fn assert_send<T: Send>() {}
/// assert_send::<ddss::Solver>();
/// ```
///
/// ```compile_fail,E0277
/// fn assert_sync<T: Sync>() {}
/// assert_sync::<ddss::Solver>();
/// ```
pub struct Solver(
    #[allow(dead_code)] ReentrantMutexGuard<'static, ()>,
    // The guard alone leaks Sync (and its !Send lives in lock_api
    // internals); *mut () pins !Send + !Sync while preserving unwind safety.
    PhantomData<*mut ()>,
);

impl Solver {
    /// Acquire exclusive access to the ddss solver, blocking until available
    ///
    /// `threads` is the maximum size of ddss's global thread pool:
    ///
    /// - `Some(n)` caps the pool at `n` threads. ddss clamps the effective
    ///   size to the detected core count (and may shave off more under
    ///   extreme memory pressure); observe the result via
    ///   [`SystemInfo::num_threads`].
    /// - `None` means no maximum: auto-detect all cores. This is enforced
    ///   like any other request, so `lock(None)` after a capped lock uncaps
    ///   the pool.
    ///
    /// The setting is process-global and the last lock wins: whenever a
    /// lock requests a different value than the one last applied, the pool
    /// is torn down and rebuilt at the new size together with its
    /// per-thread transposition tables — correct, but expensive, so prefer
    /// one consistent value per process. Repeating the applied value is
    /// free (requests are compared by requested value, not by effective
    /// pool size), and [`system_info`], [`calculate_par`], and
    /// [`calculate_pars`] never alter the setting (they only auto-size a
    /// pool that nothing has configured yet). One ddss quirk: shrinking to
    /// exactly one thread parks the existing pool threads instead of
    /// joining them; they are reclaimed by the next larger request.
    ///
    /// Capping is not pinning: to keep a macOS process on E-cores, combine a
    /// cap with OS scheduling policy such as `taskpolicy -b` or a background
    /// QoS class. The cap only prevents oversubscribing the cores the OS
    /// grants.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::num::NonZero;
    /// use ddss::Solver;
    ///
    /// let _solver = Solver::lock(NonZero::new(2));
    /// let info = ddss::system_info();
    /// assert_eq!(info.num_threads(), 2.min(info.num_cores()));
    /// ```
    #[must_use]
    pub fn lock(threads: Option<NonZero<usize>>) -> Self {
        Self(lock_pool(Some(to_request(threads))), PhantomData)
    }

    /// Try to acquire exclusive access to the ddss solver without blocking
    ///
    /// `threads` behaves as in [`Solver::lock`]. Returns `None` if the
    /// solver is currently in use by another thread; in that case nothing
    /// happens — in particular, the thread request is not applied.
    #[must_use]
    pub fn try_lock(threads: Option<NonZero<usize>>) -> Option<Self> {
        let guard = THREAD_POOL.try_lock()?;
        apply_max_threads(&guard, Some(to_request(threads)));
        Some(Self(guard, PhantomData))
    }

    /// Solve a single deal with [`sys::CalcDDtable`]
    ///
    /// # Panics
    ///
    /// Not expected — panics here are bugs. See the module-level panic policy.
    ///
    /// # Examples
    ///
    /// ```
    /// use contract_bridge::{FullDeal, Seat, Strain};
    /// use ddss::Solver;
    ///
    /// # fn main() -> Result<(), Box<dyn core::error::Error>> {
    /// // Each player holds a 13-card straight flush in one suit.
    /// let deal: FullDeal = "N:AKQJT98765432... .AKQJT98765432.. \
    ///                       ..AKQJT98765432. ...AKQJT98765432".parse()?;
    /// let solver = Solver::lock(None);
    /// let tricks = solver.solve_deal(deal);
    /// // North holds all the spades, so North or South declaring spades
    /// // draws trumps and takes every trick.
    /// assert_eq!(u8::from(tricks[Strain::Spades].get(Seat::North)), 13);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn solve_deal(&self, deal: FullDeal) -> TrickCountTable {
        let mut result = sys::ddTableResults::default();
        let table_deal = tricks::dd_table_deal_from(deal);
        // SAFETY: `table_deal` and `result` are valid, fully initialized
        // structs; the global lock held by `self` serializes ddss access.
        let status = unsafe { sys::CalcDDtable(table_deal, &raw mut result) };
        check(status);
        result.into()
    }

    /// Solve deals with a single call of [`sys::CalcAllTables`]
    ///
    /// - `deals`: A slice of deals to solve
    /// - `flags`: Flags of strains to solve for
    ///
    /// # Safety
    ///
    /// 1. **Thread-unsafe:** The caller must ensure that no other thread is
    ///    calling any ddss function while this function is running.  This is
    ///    automatically guaranteed by holding a `Solver`.
    /// 2. `deals.len() * popcount(flags)` must not exceed
    ///    [`sys::MAXNOOFBOARDS`].
    unsafe fn solve_deal_segment(deals: &[FullDeal], flags: StrainFlags) -> Box<sys::ddTablesRes> {
        debug_assert!(
            deals.len() * flags.bits().count_ones() as usize <= sys::MAXNOOFBOARDS as usize
        );
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let deal_count = deals.len() as c_int;
        // SAFETY for `pack`, `res`, and `pars`: ddss-sys's bindgen-generated
        // `Default` impls for these FFI packs are `write_bytes(_, 0, 1)`; all
        // fields are `c_int`/`c_uint`/`c_char` and arrays thereof, so the
        // all-zero bit pattern is a valid value. Boxing through `Box::default()`
        // would materialize a stack temporary (~1 MB across the three packs) at
        // opt-level 0, overflowing Windows' default thread stack.
        let mut pack: Box<sys::ddTableDeals> = unsafe { Box::new_zeroed().assume_init() };
        pack.noOfTables = deal_count;

        for (i, &deal) in deals.iter().enumerate() {
            pack.deals[i] = tricks::dd_table_deal_from(deal);
        }

        let mut filter = [
            c_int::from(!flags.contains(StrainFlags::SPADES)),
            c_int::from(!flags.contains(StrainFlags::HEARTS)),
            c_int::from(!flags.contains(StrainFlags::DIAMONDS)),
            c_int::from(!flags.contains(StrainFlags::CLUBS)),
            c_int::from(!flags.contains(StrainFlags::NOTRUMP)),
        ];
        let mut res: Box<sys::ddTablesRes> = unsafe { Box::new_zeroed().assume_init() };
        let mut pars: Box<sys::allParResults> = unsafe { Box::new_zeroed().assume_init() };
        // SAFETY: all pointers are valid for the duration of the call;
        // caller upholds the thread-exclusion invariant.
        let status = unsafe {
            sys::CalcAllTables(
                &raw mut *pack,
                -1,
                filter.as_mut_ptr(),
                &raw mut *res,
                &raw mut *pars,
            )
        };
        check(status);
        res
    }

    /// Solve deals in batch, fanning out across the ddss thread pool
    ///
    /// - `deals`: A slice of deals to solve
    /// - `flags`: Flags of strains to solve for (must be non-empty by
    ///   construction)
    ///
    /// # Panics
    ///
    /// Not expected — panics here are bugs. See the module-level panic policy.
    #[must_use]
    pub fn solve_deals(
        &self,
        deals: &[FullDeal],
        flags: NonEmptyStrainFlags,
    ) -> Vec<TrickCountTable> {
        let flags = flags.get();
        let chunk_size = (sys::MAXNOOFBOARDS / flags.bits().count_ones()) as usize;
        let mut tables = Vec::with_capacity(deals.len());
        for chunk in deals.chunks(chunk_size) {
            // SAFETY: the lock held by `self` serializes ddss access; chunking
            // keeps `chunk.len() * popcount(flags)` at or below MAXNOOFBOARDS.
            let res = unsafe { Self::solve_deal_segment(chunk, flags) };
            tables.extend(
                res.results[..chunk.len()]
                    .iter()
                    .copied()
                    .map(TrickCountTable::from),
            );
        }
        tables
    }

    /// Solve a single board with [`sys::SolveBoard`]
    ///
    /// # Panics
    ///
    /// Not expected — panics here are bugs. See the module-level panic policy.
    #[must_use]
    pub fn solve_board(&self, objective: &Objective) -> FoundPlays {
        let deal = sys::deal::from(objective.board.clone());
        let mut result = sys::futureTricks::default();
        // SAFETY: `deal` and `result` are valid; the global lock held by
        // `self` serializes ddss access.
        let status = unsafe {
            sys::SolveBoard(
                deal,
                objective.target.target(),
                objective.target.solutions(),
                0,
                &raw mut result,
                0,
            )
        };
        check(status);
        FoundPlays::from(result)
    }

    /// Solve boards with a single call of [`sys::SolveAllBoardsBin`]
    ///
    /// # Safety
    ///
    /// 1. **Thread-unsafe:** The caller must hold a `Solver` so no other
    ///    thread can enter ddss concurrently.
    /// 2. `args.len()` must not exceed [`sys::MAXNOOFBOARDS`].
    unsafe fn solve_board_segment(args: &[Objective]) -> Box<sys::solvedBoards> {
        debug_assert!(args.len() <= sys::MAXNOOFBOARDS as usize);
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let board_count = args.len() as c_int;
        // SAFETY for `pack` and `res`: ddss-sys's bindgen-generated `Default`
        // impls for these FFI packs are `write_bytes(_, 0, 1)`; all fields are
        // `c_int`/`c_uint`/`c_char` and arrays thereof, so the all-zero bit
        // pattern is a valid value. Boxing through `Box::default()` would
        // materialize a stack temporary (~1.6 MB across the two packs) at
        // opt-level 0, overflowing Windows' default thread stack.
        let mut pack: Box<sys::boards> = unsafe { Box::new_zeroed().assume_init() };
        pack.noOfBoards = board_count;

        for (i, obj) in args.iter().enumerate() {
            pack.deals[i] = sys::deal::from(obj.board.clone());
            pack.target[i] = obj.target.target();
            pack.solutions[i] = obj.target.solutions();
        }
        let mut res: Box<sys::solvedBoards> = unsafe { Box::new_zeroed().assume_init() };
        // SAFETY: caller upholds thread exclusion; pointers are valid.
        let status = unsafe { sys::SolveAllBoardsBin(&raw mut *pack, &raw mut *res) };
        check(status);
        res
    }

    /// Solve boards in batch, fanning out across the ddss thread pool
    ///
    /// # Panics
    ///
    /// Not expected — panics here are bugs. See the module-level panic policy.
    #[must_use]
    pub fn solve_boards(&self, args: &[Objective]) -> Vec<FoundPlays> {
        let mut solutions = Vec::with_capacity(args.len());
        for chunk in args.chunks(sys::MAXNOOFBOARDS as usize) {
            // SAFETY: the lock held by `self` serializes ddss access; chunking
            // keeps `chunk.len()` at or below MAXNOOFBOARDS.
            let res = unsafe { Self::solve_board_segment(chunk) };
            solutions.extend(
                res.solvedBoard[..chunk.len()]
                    .iter()
                    .copied()
                    .map(FoundPlays::from),
            );
        }
        solutions
    }

    /// Trace DD trick counts before and after each played card with
    /// [`sys::AnalysePlayBin`]
    ///
    /// # Panics
    ///
    /// Not expected — panics here are bugs. See the module-level panic policy.
    #[must_use]
    pub fn analyse_play(&self, trace: &PlayTrace) -> PlayAnalysis {
        let deal = sys::deal::from(trace.board.clone());
        let play = PlayTraceBin::from(&trace.cards);
        let mut result = sys::solvedPlay::default();
        // SAFETY: all values are valid; the global lock serializes ddss access.
        let status = unsafe { sys::AnalysePlayBin(deal, play.0, &raw mut result, 0) };
        check(status);
        PlayAnalysis::from(result)
    }

    /// Trace DD trick counts for many plays, sharing the ddss thread pool
    ///
    /// Internally loops over [`Solver::analyse_play`]; ddss's per-call thread
    /// pool still provides intra-call parallelism.
    ///
    /// # Panics
    ///
    /// Not expected — panics here are bugs. See the module-level panic policy.
    #[must_use]
    pub fn analyse_plays(&self, traces: &[PlayTrace]) -> Vec<PlayAnalysis> {
        traces.iter().map(|t| self.analyse_play(t)).collect()
    }
}

/// Get information about the underlying ddss library
///
/// Acquires the global ddss lock for the duration of the call. Safe to call
/// from any thread, including one that already holds a [`Solver`].
#[must_use]
pub fn system_info() -> SystemInfo {
    let _guard = lock_pool(None);
    let mut inner = MaybeUninit::uninit();
    // SAFETY: `GetDDSInfo` writes a fully-initialized DDSInfo into the pointer.
    unsafe { sys::GetDDSInfo(inner.as_mut_ptr()) };
    // SAFETY: `inner` was just initialized by `GetDDSInfo`.
    SystemInfo(unsafe { inner.assume_init() })
}

#[cfg(test)]
mod tests {
    use super::{UNINIT, thread_target};

    #[test]
    fn thread_target_applies_the_first_request() {
        assert_eq!(thread_target(None, UNINIT), Some(0));
        assert_eq!(thread_target(Some(0), UNINIT), Some(0));
        assert_eq!(thread_target(Some(6), UNINIT), Some(6));
    }

    #[test]
    fn thread_target_skips_matching_or_indifferent_requests() {
        assert_eq!(thread_target(None, 0), None);
        assert_eq!(thread_target(None, 6), None);
        assert_eq!(thread_target(Some(0), 0), None);
        assert_eq!(thread_target(Some(6), 6), None);
    }

    #[test]
    fn thread_target_resizes_on_differing_requests() {
        assert_eq!(thread_target(Some(2), 6), Some(2));
        assert_eq!(thread_target(Some(6), 2), Some(6));
        assert_eq!(thread_target(Some(6), 0), Some(6));
        assert_eq!(thread_target(Some(0), 6), Some(0));
    }
}

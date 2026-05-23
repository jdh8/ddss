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
use core::mem::MaybeUninit;
use std::sync::LazyLock;

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
    let _guard = THREAD_POOL.lock();
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
    let _guard = THREAD_POOL.lock();
    let mut pars = [sys::parResultsMaster::default(); 2];
    let status = unsafe { sys::SidesParBin(&mut tricks.into(), &raw mut pars[0], vul.to_sys()) };
    check(status);
    pars.map(Into::into)
}

/// Global reentrant lock guarding ddss's non-reentrant C entry points.
///
/// Reentrant within a single thread so that helpers like [`calculate_par`]
/// and [`system_info`] can be called from a thread that already holds a
/// [`Solver`]; different threads still block. `SetMaxThreads(0)` is called
/// on first acquisition to spin up ddss's internal thread pool.
static THREAD_POOL: LazyLock<ReentrantMutex<()>> = LazyLock::new(|| {
    // SAFETY: ddss accepts a thread count and configures its pool. Passing 0
    // asks ddss to auto-detect.
    unsafe { sys::SetMaxThreads(0) };
    ReentrantMutex::new(())
});

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
/// `Solver` is `!Send` because [`ReentrantMutexGuard`] is `!Send` — the lock
/// must be released on the same OS thread that acquired it.
pub struct Solver(#[allow(dead_code)] ReentrantMutexGuard<'static, ()>);

impl Solver {
    /// Acquire exclusive access to the ddss solver, blocking until available
    #[must_use]
    pub fn lock() -> Self {
        Self(THREAD_POOL.lock())
    }

    /// Try to acquire exclusive access to the ddss solver without blocking
    ///
    /// Returns `None` if the solver is currently in use.
    #[must_use]
    pub fn try_lock() -> Option<Self> {
        THREAD_POOL.try_lock().map(Self)
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
    /// let solver = Solver::lock();
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
        let mut pack: Box<sys::ddTableDeals> = Box::default();
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
        let mut res: Box<sys::ddTablesRes> = Box::default();
        let mut pars: Box<sys::allParResults> = Box::default();
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
        let mut pack: Box<sys::boards> = Box::default();
        pack.noOfBoards = board_count;

        for (i, obj) in args.iter().enumerate() {
            pack.deals[i] = sys::deal::from(obj.board.clone());
            pack.target[i] = obj.target.target();
            pack.solutions[i] = obj.target.solutions();
        }
        let mut res: Box<sys::solvedBoards> = Box::default();
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
    let _guard = THREAD_POOL.lock();
    let mut inner = MaybeUninit::uninit();
    // SAFETY: `GetDDSInfo` writes a fully-initialized DDSInfo into the pointer.
    unsafe { sys::GetDDSInfo(inner.as_mut_ptr()) };
    // SAFETY: `inner` was just initialized by `GetDDSInfo`.
    SystemInfo(unsafe { inner.assume_init() })
}

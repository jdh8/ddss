use arrayvec::ArrayVec;
use contract_bridge::{Builder, Card, Contract, Hand, Holding, Penalty, Rank, Seat, Strain, Suit};
use ddss::*;
use semver::Version;

/// Everyone has a 13-card straight flush, and the par is 7SW=.
#[test]
fn solve_four_13_card_straight_flushes() -> Result<(), Builder> {
    const DEAL: Builder = Builder::new()
        .north(Hand::new(
            Holding::ALL,
            Holding::EMPTY,
            Holding::EMPTY,
            Holding::EMPTY,
        ))
        .east(Hand::new(
            Holding::EMPTY,
            Holding::ALL,
            Holding::EMPTY,
            Holding::EMPTY,
        ))
        .south(Hand::new(
            Holding::EMPTY,
            Holding::EMPTY,
            Holding::ALL,
            Holding::EMPTY,
        ))
        .west(Hand::new(
            Holding::EMPTY,
            Holding::EMPTY,
            Holding::EMPTY,
            Holding::ALL,
        ));
    const SOLUTION: TrickCountTable = TrickCountTable([
        TrickCountRow::new(13, 0, 13, 0),
        TrickCountRow::new(0, 13, 0, 13),
        TrickCountRow::new(13, 0, 13, 0),
        TrickCountRow::new(0, 13, 0, 13),
        TrickCountRow::new(0, 0, 0, 0),
    ]);
    const CONTRACT: Contract = Contract::new(7, Strain::Spades, Penalty::Undoubled);
    const CONTRACTS: [ParContract; 2] = [
        ParContract {
            contract: CONTRACT,
            declarer: Seat::East,
            overtricks: 0,
        },
        ParContract {
            contract: CONTRACT,
            declarer: Seat::West,
            overtricks: 0,
        },
    ];
    let ns = Par {
        score: -2210,
        contracts: CONTRACTS.to_vec(),
    };
    let ew = Par {
        score: 2210,
        contracts: CONTRACTS.to_vec(),
    };
    assert_eq!(Solver::lock().solve_deal(DEAL.build_full()?), SOLUTION);

    let pars = calculate_pars(SOLUTION, Vulnerability::all());
    assert!(pars[0].equivalent(&ns));
    assert!(pars[1].equivalent(&ew));
    Ok(())
}

/// Defenders can cash 8 tricks in every strain.
///
/// This example is taken from
/// <http://bridge.thomasoandrews.com/deals/parzero/>.
#[test]
fn solve_par_5_tricks() -> Result<(), Builder> {
    const AKQJ: Holding = Holding::from_bits_truncate(0xF << 11);
    const T987: Holding = Holding::from_bits_truncate(0xF << 7);
    const XXXX: Holding = Holding::from_bits_truncate(0xF << 3);
    const X: Holding = Holding::from_bits_truncate(1 << 2);
    const DEAL: Builder = Builder::new()
        .north(Hand::new(T987, XXXX, X, AKQJ))
        .east(Hand::new(X, AKQJ, T987, XXXX))
        .south(Hand::new(XXXX, T987, AKQJ, X))
        .west(Hand::new(AKQJ, X, XXXX, T987));
    const SOLUTION: TrickCountTable = TrickCountTable([TrickCountRow::new(5, 5, 5, 5); 5]);
    const PAR: Par = Par {
        score: 0,
        contracts: Vec::new(),
    };
    assert_eq!(Solver::lock().solve_deal(DEAL.build_full()?), SOLUTION);

    let pars = calculate_pars(SOLUTION, Vulnerability::all());
    assert!(pars[0].equivalent(&PAR));
    assert!(pars[1].equivalent(&PAR));
    Ok(())
}

/// A symmetric deal where everyone makes 1NT but no suit contract
///
/// This example is taken from
/// <http://www.rpbridge.net/7a23.htm#2>.
#[test]
fn solve_everyone_makes_1nt() -> Result<(), Builder> {
    const A54: Holding = Holding::from_bits_truncate(0b100_0000_0011_0000);
    const QJ32: Holding = Holding::from_bits_truncate(0b001_1000_0000_1100);
    const K976: Holding = Holding::from_bits_truncate(0b010_0010_1100_0000);
    const T8: Holding = Holding::from_bits_truncate(0b000_0101_0000_0000);
    const DEAL: Builder = Builder::new()
        .north(Hand::new(A54, QJ32, K976, T8))
        .east(Hand::new(T8, A54, QJ32, K976))
        .south(Hand::new(K976, T8, A54, QJ32))
        .west(Hand::new(QJ32, K976, T8, A54));
    const SUIT: TrickCountRow = TrickCountRow::new(6, 6, 6, 6);
    const NT: TrickCountRow = TrickCountRow::new(7, 7, 7, 7);
    const SOLUTION: TrickCountTable = TrickCountTable([SUIT, SUIT, SUIT, SUIT, NT]);
    const CONTRACT: Contract = Contract::new(1, Strain::Notrump, Penalty::Undoubled);
    assert_eq!(Solver::lock().solve_deal(DEAL.build_full()?), SOLUTION);

    let ns = Par {
        score: 90,
        contracts: vec![
            ParContract {
                contract: CONTRACT,
                declarer: Seat::North,
                overtricks: 0,
            },
            ParContract {
                contract: CONTRACT,
                declarer: Seat::South,
                overtricks: 0,
            },
        ],
    };
    let ew = Par {
        score: 90,
        contracts: vec![
            ParContract {
                contract: CONTRACT,
                declarer: Seat::East,
                overtricks: 0,
            },
            ParContract {
                contract: CONTRACT,
                declarer: Seat::West,
                overtricks: 0,
            },
        ],
    };
    let pars = calculate_pars(SOLUTION, Vulnerability::all());
    assert!(pars[0].equivalent(&ns));
    assert!(pars[1].equivalent(&ew));
    Ok(())
}

/// `solve_board` scores agree with the double-dummy table for the same deal.
#[test]
fn solve_board_score_matches_dd_table() -> anyhow::Result<()> {
    const A54: Holding = Holding::from_bits_truncate(0b100_0000_0011_0000);
    const QJ32: Holding = Holding::from_bits_truncate(0b001_1000_0000_1100);
    const K976: Holding = Holding::from_bits_truncate(0b010_0010_1100_0000);
    const T8: Holding = Holding::from_bits_truncate(0b000_0101_0000_0000);
    const DEAL: Builder = Builder::new()
        .north(Hand::new(A54, QJ32, K976, T8))
        .east(Hand::new(T8, A54, QJ32, K976))
        .south(Hand::new(K976, T8, A54, QJ32))
        .west(Hand::new(QJ32, K976, T8, A54));
    let solver = Solver::lock();
    let full = DEAL
        .build_full()
        .map_err(|_| anyhow::anyhow!("DEAL is not a full deal"))?;
    let partial = DEAL
        .build_partial()
        .map_err(|_| anyhow::anyhow!("DEAL is not a valid partial deal"))?;
    let tricks = solver.solve_deal(full);
    let found = solver.solve_board(&Objective {
        board: Board::try_new(partial, CurrentTrick::new(Strain::Notrump, Seat::North))?,
        target: Target::Any(None),
    });
    let expected = 13 - u8::from(tricks[Strain::Notrump].get(Seat::North.rho()));
    assert!(!found.plays.is_empty());
    assert_eq!(u8::from(found.plays[0].score), expected);
    Ok(())
}

/// `solve_boards` returns the same plays as individual `solve_board` calls.
#[test]
fn solve_boards_matches_solve_board() -> anyhow::Result<()> {
    const A54: Holding = Holding::from_bits_truncate(0b100_0000_0011_0000);
    const QJ32: Holding = Holding::from_bits_truncate(0b001_1000_0000_1100);
    const K976: Holding = Holding::from_bits_truncate(0b010_0010_1100_0000);
    const T8: Holding = Holding::from_bits_truncate(0b000_0101_0000_0000);
    const DEAL: Builder = Builder::new()
        .north(Hand::new(A54, QJ32, K976, T8))
        .east(Hand::new(T8, A54, QJ32, K976))
        .south(Hand::new(K976, T8, A54, QJ32))
        .west(Hand::new(QJ32, K976, T8, A54));
    let solver = Solver::lock();
    let partial = DEAL
        .build_partial()
        .map_err(|_| anyhow::anyhow!("DEAL is not a valid partial deal"))?;
    let obj = Objective {
        board: Board::try_new(partial, CurrentTrick::new(Strain::Notrump, Seat::North))?,
        target: Target::Any(None),
    };
    let single = solver.solve_board(&obj);
    let batch = solver.solve_boards(&[obj]);
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0].plays, single.plays);
    Ok(())
}

/// `solve_deals` must agree with sequential `solve_deal` on a varied batch.
#[test]
fn solve_deals_batch_matches_sequential() -> Result<(), Builder> {
    // Three hand-crafted deals; using fixed inputs keeps the test deterministic
    // without pulling in a randomness dep.
    const DEALS: [Builder; 3] = [
        Builder::new()
            .north(Hand::new(
                Holding::ALL,
                Holding::EMPTY,
                Holding::EMPTY,
                Holding::EMPTY,
            ))
            .east(Hand::new(
                Holding::EMPTY,
                Holding::ALL,
                Holding::EMPTY,
                Holding::EMPTY,
            ))
            .south(Hand::new(
                Holding::EMPTY,
                Holding::EMPTY,
                Holding::ALL,
                Holding::EMPTY,
            ))
            .west(Hand::new(
                Holding::EMPTY,
                Holding::EMPTY,
                Holding::EMPTY,
                Holding::ALL,
            )),
        Builder::new()
            .north(Hand::new(
                Holding::ALL,
                Holding::EMPTY,
                Holding::EMPTY,
                Holding::EMPTY,
            ))
            .south(Hand::new(
                Holding::EMPTY,
                Holding::ALL,
                Holding::EMPTY,
                Holding::EMPTY,
            ))
            .east(Hand::new(
                Holding::EMPTY,
                Holding::EMPTY,
                Holding::ALL,
                Holding::EMPTY,
            ))
            .west(Hand::new(
                Holding::EMPTY,
                Holding::EMPTY,
                Holding::EMPTY,
                Holding::ALL,
            )),
        Builder::new()
            .north(Hand::new(
                Holding::from_bits_truncate(0xF << 11),
                Holding::from_bits_truncate(0xF << 7),
                Holding::from_bits_truncate(0xF << 3),
                Holding::from_bits_truncate(1 << 2),
            ))
            .east(Hand::new(
                Holding::from_bits_truncate(1 << 2),
                Holding::from_bits_truncate(0xF << 11),
                Holding::from_bits_truncate(0xF << 7),
                Holding::from_bits_truncate(0xF << 3),
            ))
            .south(Hand::new(
                Holding::from_bits_truncate(0xF << 3),
                Holding::from_bits_truncate(1 << 2),
                Holding::from_bits_truncate(0xF << 11),
                Holding::from_bits_truncate(0xF << 7),
            ))
            .west(Hand::new(
                Holding::from_bits_truncate(0xF << 7),
                Holding::from_bits_truncate(0xF << 3),
                Holding::from_bits_truncate(1 << 2),
                Holding::from_bits_truncate(0xF << 11),
            )),
    ];
    let deals: Vec<_> = DEALS
        .iter()
        .map(|b| b.build_full())
        .collect::<Result<_, _>>()?;
    let solver = Solver::lock();
    let batch = solver.solve_deals(&deals, NonEmptyStrainFlags::ALL);
    let sequential: Vec<_> = deals.iter().map(|&d| solver.solve_deal(d)).collect();
    assert_eq!(batch, sequential);
    Ok(())
}

/// `analyse_play` with an empty trace returns just the starting DD value.
#[test]
fn analyse_play_empty_trace_complements_solve_board() -> anyhow::Result<()> {
    const A54: Holding = Holding::from_bits_truncate(0b100_0000_0011_0000);
    const QJ32: Holding = Holding::from_bits_truncate(0b001_1000_0000_1100);
    const K976: Holding = Holding::from_bits_truncate(0b010_0010_1100_0000);
    const T8: Holding = Holding::from_bits_truncate(0b000_0101_0000_0000);
    const DEAL: Builder = Builder::new()
        .north(Hand::new(A54, QJ32, K976, T8))
        .east(Hand::new(T8, A54, QJ32, K976))
        .south(Hand::new(K976, T8, A54, QJ32))
        .west(Hand::new(QJ32, K976, T8, A54));
    let partial = DEAL
        .build_partial()
        .map_err(|_| anyhow::anyhow!("DEAL is not a valid partial deal"))?;
    let board = Board::try_new(partial, CurrentTrick::new(Strain::Notrump, Seat::North))?;
    let solver = Solver::lock();
    let found = solver.solve_board(&Objective {
        board: board.clone(),
        target: Target::Any(None),
    });
    let analysis = solver.analyse_play(&PlayTrace {
        board,
        cards: ArrayVec::new(),
    });
    assert_eq!(analysis.tricks.len(), 1);
    assert_eq!(
        u8::from(analysis.tricks[0]) + u8::from(found.plays[0].score),
        13,
    );
    Ok(())
}

/// Straight-flush deal, NT contract: opening lead of ♣A — declarer (West) takes
/// zero, and the DD value must be 0 both before and after the lead.
#[test]
fn analyse_play_straight_flush_declarer_takes_zero() -> anyhow::Result<()> {
    const DEAL: Builder = Builder::new()
        .north(Hand::new(
            Holding::ALL,
            Holding::EMPTY,
            Holding::EMPTY,
            Holding::EMPTY,
        ))
        .east(Hand::new(
            Holding::EMPTY,
            Holding::ALL,
            Holding::EMPTY,
            Holding::EMPTY,
        ))
        .south(Hand::new(
            Holding::EMPTY,
            Holding::EMPTY,
            Holding::ALL,
            Holding::EMPTY,
        ))
        .west(Hand::new(
            Holding::EMPTY,
            Holding::EMPTY,
            Holding::EMPTY,
            Holding::ALL,
        ));
    let mut cards = ArrayVec::<Card, 52>::new();
    cards.push(Card {
        suit: Suit::Clubs,
        rank: Rank::A,
    });
    let partial = DEAL
        .build_partial()
        .map_err(|_| anyhow::anyhow!("DEAL is not a valid partial deal"))?;
    let board = Board::try_new(partial, CurrentTrick::new(Strain::Notrump, Seat::North))?;
    let analysis = Solver::lock().analyse_play(&PlayTrace { board, cards });
    assert_eq!(analysis.tricks.len(), 2);
    assert!(analysis.tricks.iter().all(|&t| u8::from(t) == 0));
    Ok(())
}

#[test]
fn system_info_version_is_2_9_0() {
    let info = system_info();
    assert_eq!(info.version(), Version::new(2, 9, 0));
}

#[test]
fn system_info_platform_matches_os() {
    let platform = match () {
        () if cfg!(target_os = "linux") => Platform::Linux,
        () if cfg!(target_os = "macos") => Platform::Apple,
        () if cfg!(target_os = "cygwin") => Platform::Cygwin,
        () if cfg!(target_os = "windows") => Platform::Windows,
        () => return,
    };
    let info = system_info();
    assert_eq!(info.platform(), platform);
}

#[test]
fn system_info_num_bits_matches_target() {
    let num_bits: u32 = match () {
        () if cfg!(target_pointer_width = "64") => 64,
        () if cfg!(target_pointer_width = "32") => 32,
        () if cfg!(target_pointer_width = "16") => 16,
        () => return,
    };
    let info = system_info();
    assert_eq!(info.num_bits(), num_bits);
}

#[test]
fn system_info_compiler_is_known() {
    assert!(!matches!(system_info().compiler(), Compiler::Unknown(_)));
}

#[test]
fn system_info_threading_is_stl() {
    // Force the global pool to initialize so `noOfThreads`/`threadSizes` are
    // populated when other tests query them in parallel.
    let _guard = Solver::lock();
    assert_eq!(system_info().threading(), Threading::STL);
}

#[test]
fn system_info_num_cores_is_positive() {
    assert!(system_info().num_cores() > 0);
}

#[test]
fn trick_count_try_new_rejects_out_of_range() {
    assert_eq!(TrickCount::try_new(14), Err(InvalidTrickCount));
    assert_eq!(TrickCount::try_new(255), Err(InvalidTrickCount));
    assert!(TrickCount::try_new(0).is_ok());
    assert!(TrickCount::try_new(13).is_ok());
}

#[test]
fn trick_count_row_try_new_rejects_out_of_range() {
    assert_eq!(TrickCountRow::try_new(14, 0, 0, 0), Err(InvalidTrickCount));
    assert_eq!(TrickCountRow::try_new(0, 14, 0, 0), Err(InvalidTrickCount));
    assert_eq!(TrickCountRow::try_new(0, 0, 14, 0), Err(InvalidTrickCount));
    assert_eq!(TrickCountRow::try_new(0, 0, 0, 14), Err(InvalidTrickCount));
    assert!(TrickCountRow::try_new(13, 13, 13, 13).is_ok());
    assert!(TrickCountRow::try_new(0, 0, 0, 0).is_ok());
}

fn subset_from(
    north: impl IntoIterator<Item = Card>,
    east: impl IntoIterator<Item = Card>,
    south: impl IntoIterator<Item = Card>,
    west: impl IntoIterator<Item = Card>,
) -> contract_bridge::PartialDeal {
    Builder::new()
        .north(Hand::from_iter(north))
        .east(Hand::from_iter(east))
        .south(Hand::from_iter(south))
        .west(Hand::from_iter(west))
        .build_partial()
        .expect("caller supplies ≤13 pairwise-disjoint cards per hand")
}

const fn c(suit: Suit, rank: u8) -> Card {
    Card {
        suit,
        rank: Rank::new(rank),
    }
}

/// East fails to follow North's spade lead despite still holding a spade.
#[test]
fn board_try_new_detects_revoke_on_second_card() -> Result<(), CurrentTrickError> {
    let remaining = subset_from(
        [c(Suit::Hearts, 3), c(Suit::Hearts, 4), c(Suit::Hearts, 5)],
        [c(Suit::Spades, 13), c(Suit::Hearts, 6), c(Suit::Hearts, 7)],
        [
            c(Suit::Diamonds, 2),
            c(Suit::Diamonds, 3),
            c(Suit::Diamonds, 4),
            c(Suit::Diamonds, 5),
        ],
        [
            c(Suit::Clubs, 2),
            c(Suit::Clubs, 3),
            c(Suit::Clubs, 4),
            c(Suit::Clubs, 5),
        ],
    );
    let played = [c(Suit::Spades, 14), c(Suit::Hearts, 2)];
    assert_eq!(
        Board::try_new(
            remaining,
            CurrentTrick::from_slice(Strain::Notrump, Seat::North, &played)?,
        ),
        Err(BoardError::Revoke {
            position: RevokePosition::Second
        })
    );
    Ok(())
}

/// `CurrentTrick::from_slice` rejects more than three cards.
#[test]
fn current_trick_from_slice_rejects_overlong() {
    let played = [
        c(Suit::Spades, 14),
        c(Suit::Spades, 13),
        c(Suit::Spades, 12),
        c(Suit::Spades, 11),
    ];
    assert_eq!(
        CurrentTrick::from_slice(Strain::Notrump, Seat::North, &played),
        Err(CurrentTrickError::TooManyPlayed),
    );
}

/// `CurrentTrick::from_slice` rejects duplicated cards.
#[test]
fn current_trick_from_slice_rejects_duplicate() {
    let played = [c(Suit::Spades, 14), c(Suit::Spades, 14)];
    assert_eq!(
        CurrentTrick::from_slice(Strain::Notrump, Seat::North, &played),
        Err(CurrentTrickError::DuplicatePlayedCard),
    );
}

#[test]
fn vulnerability_display_fromstr_roundtrip() {
    for v in [
        Vulnerability::NONE,
        Vulnerability::NS,
        Vulnerability::EW,
        Vulnerability::ALL,
    ] {
        assert_eq!(v.to_string().parse::<Vulnerability>().unwrap(), v);
    }
}

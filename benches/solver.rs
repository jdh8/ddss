use contract_bridge::deck::full_deal;
use core::hint::black_box;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ddss::{NonEmptyStrainFlags, Solver};
use rand::SeedableRng;
use rand::rngs::SmallRng;

fn bench_solve_deal_single(c: &mut Criterion) {
    let mut rng = SmallRng::seed_from_u64(0);
    let solver = Solver::lock();
    c.bench_function("solve_deal_single", |b| {
        b.iter_batched(
            || full_deal(&mut rng),
            |deal| black_box(solver.solve_deal(black_box(deal))),
            BatchSize::SmallInput,
        );
    });
}

fn bench_solve_deals_batch_32(c: &mut Criterion) {
    let mut rng = SmallRng::seed_from_u64(1);
    let deals: Vec<_> = (0..32).map(|_| full_deal(&mut rng)).collect();
    let solver = Solver::lock();
    let mut group = c.benchmark_group("solve_deals_batch");
    group.sample_size(20);
    group.bench_function("32", |b| {
        b.iter(|| black_box(solver.solve_deals(black_box(&deals), NonEmptyStrainFlags::ALL)));
    });
    group.finish();
}

criterion_group!(benches, bench_solve_deal_single, bench_solve_deals_batch_32);
criterion_main!(benches);

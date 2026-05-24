use contract_bridge::deck::full_deal;
use core::hint::black_box;
use core::time::Duration;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
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

fn bench_solve_deals_batch(c: &mut Criterion) {
    let mut rng = SmallRng::seed_from_u64(1);
    let solver = Solver::lock();
    let mut group = c.benchmark_group("solve_deals_batch");
    // 10 samples (criterion's floor) + 90 s budget keeps N=1000 tractable
    // since a single iteration alone exceeds the default 5 s measurement_time.
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(90));
    for &n in &[32_usize, 200, 1000] {
        let deals: Vec<_> = (0..n).map(|_| full_deal(&mut rng)).collect();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &deals, |b, deals| {
            b.iter(|| black_box(solver.solve_deals(black_box(deals), NonEmptyStrainFlags::ALL)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_solve_deal_single, bench_solve_deals_batch);
criterion_main!(benches);

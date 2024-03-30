use bevy_math::{DVec3, Vec3};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use svo::TerrainCellData;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n-1) + fibonacci(n-2),
    }
}

fn bench(c: &mut Criterion) {
    let depth = 10;
    let svo = svo::TerrainCell::new_with_depth(depth, TerrainCellData {
        kind: svo::TerrainCellKind::Air,
        distance: 5.,
    });

    c.bench_with_input(BenchmarkId::new("cell.sample", depth), &svo, |b, i| {
        b.iter(|| i.sample::<f32>(Vec3::new(0.5, 0.5, 0.5), depth));
        b.iter(|| i.sample::<f32>(Vec3::new(0.5, 0.75, 0.25), depth));
        b.iter(|| i.sample::<f64>(DVec3::new(0.5, 0.5, 0.25), depth));
        b.iter(|| i.sample::<f64>(DVec3::new(0.5, 0.75, 0.25), depth));
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);

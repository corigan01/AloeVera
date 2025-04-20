use boolvec::{BoolVec, CompressedBool};
use criterion::{
    AxisScale, BenchmarkId, Criterion, PlotConfiguration, criterion_group, criterion_main,
};
use rand::random_range;

fn criterion_benchmark(c: &mut Criterion) {
    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);

    // let mut group = c.benchmark_group("Bool Iterations");
    // group.plot_config(plot_config.clone());
    // for iterations in [1, 1_000, 10_000, 100_000, 1_000_000] {
    //     group.throughput(criterion::Throughput::Elements(iterations as u64));
    //     group.bench_with_input(
    //         BenchmarkId::new("Compressed Bool", iterations),
    //         &iterations,
    //         |f, &offset| {
    //             let mut array = CompressedBool::new();

    //             f.iter(|| {
    //                 for i in 0..offset {
    //                     array.set(i, true);
    //                 }

    //                 assert_eq!(array.find_first_of(false), Some(offset));
    //                 assert_eq!(array.find_first_of(true), Some(0));

    //                 for i in 0..offset {
    //                     array.set(i, false);
    //                 }

    //                 assert_eq!(array.find_first_of(false), Some(0));
    //                 assert_eq!(array.find_first_of(true), None);
    //             });
    //         },
    //     );

    //     group.bench_with_input(
    //         BenchmarkId::new("Bool Vec", iterations),
    //         &iterations,
    //         |f, &offset| {
    //             let mut array = BoolVec::new();

    //             f.iter(|| {
    //                 for i in 0..offset {
    //                     array.set(i, true);
    //                 }

    //                 assert_eq!(array.find_first_of(false), Some(offset));
    //                 assert_eq!(array.find_first_of(true), Some(0));

    //                 for i in 0..offset {
    //                     array.set(i, false);
    //                 }

    //                 assert_eq!(array.find_first_of(false), Some(0));
    //                 assert_eq!(array.find_first_of(true), None);
    //             });
    //         },
    //     );
    // }

    // group.finish();

    // let mut group = c.benchmark_group("Bool Array Singles Benchmarks");
    // group.plot_config(plot_config.clone());
    // for offsets in [0, 1_000, 10_000, 100_000, 1_000_000] {
    //     group.bench_with_input(
    //         BenchmarkId::new("Compressed Bool", offsets),
    //         &offsets,
    //         |f, &offset| {
    //             let mut array = CompressedBool::new();

    //             f.iter(|| {
    //                 array.set(offset, true);

    //                 assert_eq!(
    //                     array.find_first_of(false),
    //                     if offset == 0 { Some(1) } else { Some(0) }
    //                 );
    //                 assert_eq!(array.find_first_of(true), Some(offset));

    //                 array.set(offset, false);

    //                 assert_eq!(array.find_first_of(false), Some(0));
    //                 assert_eq!(array.find_first_of(true), None);
    //             });
    //         },
    //     );

    //     group.bench_with_input(
    //         BenchmarkId::new("Bool Vec", offsets),
    //         &offsets,
    //         |f, &offset| {
    //             let mut array = BoolVec::new();

    //             f.iter(|| {
    //                 array.set(offset, true);

    //                 assert_eq!(
    //                     array.find_first_of(false),
    //                     if offset == 0 { Some(1) } else { Some(0) }
    //                 );
    //                 assert_eq!(array.find_first_of(true), Some(offset));

    //                 array.set(offset, false);

    //                 assert_eq!(array.find_first_of(false), Some(0));
    //                 assert_eq!(array.find_first_of(true), None);
    //             });
    //         },
    //     );
    // }
    // group.finish();

    // This is to keep both the `CompressedBool` and `BoolVec` have the same input
    let mut random_samples = Vec::new();
    for _ in 0..10_000 {
        random_samples.push(random_range(0..(1024 * 1024 * 1024)));
    }

    let mut group = c.benchmark_group("Bool Random");
    group.plot_config(plot_config.clone());
    for iterations in [1, 10, 100, 1_000, 10_000] {
        group.throughput(criterion::Throughput::Elements(iterations as u64));
        group.bench_with_input(
            BenchmarkId::new("Compressed Bool", iterations),
            &iterations,
            |f, &offset| {
                f.iter(|| {
                    let mut array = CompressedBool::new();

                    for i in 0..offset {
                        array.set(random_samples[i], true);
                    }

                    for i in 0..offset {
                        assert_eq!(array.get(random_samples[i]), true);
                    }

                    for i in 0..offset {
                        array.set(random_samples[i], false);
                    }

                    for i in 0..offset {
                        assert_eq!(array.get(random_samples[i]), false);
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("Bool Vec", iterations),
            &iterations,
            |f, &offset| {
                f.iter(|| {
                    let mut array = BoolVec::new();

                    for i in 0..offset {
                        array.set(random_samples[i], true);
                    }

                    for i in 0..offset {
                        assert_eq!(array.get(random_samples[i]), true);
                    }

                    for i in 0..offset {
                        array.set(random_samples[i], false);
                    }

                    for i in 0..offset {
                        assert_eq!(array.get(random_samples[i]), false);
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

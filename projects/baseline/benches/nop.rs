use std::arch::asm;

use criterion::{criterion_group, criterion_main, Criterion};
use criterion_energy::msr::measurement::Energy;

fn simple(c: &mut Criterion::<Energy>) {
    {
        let mut group = c.benchmark_group("nops");
        group.sampling_mode(criterion::SamplingMode::Flat);
        
        group.bench_function("nops", |bencher| {
            bencher.iter(|| {
                for _ in 0..100 {
                    unsafe {
                        asm!{
                            "nop"
                        }
                    }
                }
            });
        });
    }
}

criterion_group!(name=benches; config = Criterion::default().with_measurement(Energy); targets =  simple);
criterion_main!(benches);

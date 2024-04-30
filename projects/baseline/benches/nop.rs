use std::arch::asm;

use criterion::{criterion_group, criterion_main, Criterion};
use criterion_energy::msr::measurement::Energy;

macro_rules! nop {
    ($group:expr, $number:literal) => {
        $group.bench_function($number.to_string(), |bencher| {
            bencher.iter(|| {
                for _ in 0..$number {
                    unsafe {
                        asm!{
                            "nop"
                        }
                    }
                }
            });
        });
    };
}

fn simple(c: &mut Criterion::<Energy>) {
    {
        let mut group = c.benchmark_group("nops");
        group.sampling_mode(criterion::SamplingMode::Flat);
        
        nop!(group, 100);
        nop!(group, 1000);
        nop!(group, 10000);
        nop!(group, 100000);
        nop!(group, 1000000);
    }
}



criterion_group!(name=benches; config = Criterion::default().with_measurement(Energy); targets =  simple);
criterion_main!(benches);

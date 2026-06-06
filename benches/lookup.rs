//! Micro-benchmark for the hot lookup path.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oui_lookup::lookup;

fn bench_lookup(c: &mut Criterion) {
    // A spread of well-known, registered OUIs.
    let macs = [
        "00:11:22:33:44:55",
        "a4:83:e7:00:00:00",
        "28:cf:e9:11:22:33",
        "3c:5a:b4:de:ad:be",
        "f0:18:98:ff:ff:ff",
    ];

    c.bench_function("lookup_known", |b| {
        b.iter(|| {
            for m in &macs {
                black_box(lookup(black_box(m)));
            }
        })
    });

    c.bench_function("lookup_unknown", |b| {
        b.iter(|| black_box(lookup(black_box("FF:FF:FF:00:00:00"))))
    });
}

criterion_group!(benches, bench_lookup);
criterion_main!(benches);

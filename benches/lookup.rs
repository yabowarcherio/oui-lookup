//! Micro-benchmark for the hot lookup path.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oui_lookup::{classify, entries, lookup, lookup_oui_octets, parse_mac48, scope, search};

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

    c.bench_function("parse_and_classify", |b| {
        b.iter(|| {
            let m = parse_mac48(black_box("a4:83:e7:9c:1d:42")).unwrap();
            black_box(classify(m))
        })
    });

    c.bench_function("scope_global_unicast", |b| {
        let m = parse_mac48("a4:83:e7:9c:1d:42").unwrap();
        b.iter(|| black_box(scope(black_box(m))))
    });

    c.bench_function("scope_ipv4_multicast", |b| {
        let m = parse_mac48("01:00:5e:00:00:01").unwrap();
        b.iter(|| black_box(scope(black_box(m))))
    });

    c.bench_function("lookup_oui_octets", |b| {
        b.iter(|| black_box(lookup_oui_octets(black_box([0xA4, 0x83, 0xE7]))))
    });

    c.bench_function("lookup_unknown", |b| {
        b.iter(|| black_box(lookup(black_box("FF:FF:FF:00:00:00"))))
    });
}

fn bench_search(c: &mut Criterion) {
    c.bench_function("search_apple", |b| {
        b.iter(|| black_box(search(black_box("apple")).count()))
    });
    c.bench_function("iterate_all", |b| b.iter(|| black_box(entries().count())));
    c.bench_function("top_vendors_10", |b| {
        b.iter(|| black_box(oui_lookup::top_vendors(black_box(10)).len()))
    });
}

criterion_group!(benches, bench_lookup, bench_search);
criterion_main!(benches);

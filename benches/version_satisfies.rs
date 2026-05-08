use criterion::{black_box, criterion_group, criterion_main, Criterion};
use guroku::version::{parse_range, parse_version};

fn bench_satisfies(c: &mut Criterion) {
    let v = parse_version("1.2.5").unwrap();
    let cases: &[(&str, &str)] = &[
        ("caret", "^1.2.3"),
        ("tilde", "~1.0"),
        ("inclusive", ">=1.0 <2.0"),
        ("or", "^1 || ^2"),
        ("x_range", "1.2.x"),
        ("exact", "1.2.3"),
    ];
    for (label, range_str) in cases {
        c.bench_function(&format!("parse_then_satisfy/{label}"), |b| {
            b.iter(|| {
                let r = parse_range(black_box(range_str)).unwrap();
                let _ = r.satisfies(&v);
            });
        });
        let r = parse_range(range_str).unwrap();
        c.bench_function(&format!("satisfy_only/{label}"), |b| {
            b.iter(|| {
                let _ = black_box(&r).satisfies(black_box(&v));
            });
        });
    }
}

criterion_group!(benches, bench_satisfies);
criterion_main!(benches);

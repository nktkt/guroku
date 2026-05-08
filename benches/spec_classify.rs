use criterion::{black_box, criterion_group, criterion_main, Criterion};
use guroku::specs::classify;

fn bench_classify(c: &mut Criterion) {
    let cases: &[(&str, &str)] = &[
        ("range_caret", "^1.2.3"),
        ("range_or", "^1 || ^2"),
        ("range_x", "1.2.x"),
        ("range_inclusive", ">=1.0 <2.0"),
        ("dist_tag", "latest"),
        ("file", "file:./local-pkg"),
        ("git_https", "git+https://github.com/u/r.git"),
        (
            "git_https_revision",
            "git+https://github.com/u/r.git#v1.2.3",
        ),
        ("github_shorthand", "github:u/r#main"),
        ("git_ssh", "git+ssh://git@host/r.git"),
    ];
    for (label, input) in cases {
        c.bench_function(&format!("spec_classify/{label}"), |b| {
            b.iter(|| {
                let _ = classify(black_box(input));
            });
        });
    }
}

criterion_group!(benches, bench_classify);
criterion_main!(benches);

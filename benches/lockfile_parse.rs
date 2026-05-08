use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn build_lockfile_json(n: usize) -> String {
    let mut packages = String::new();
    for i in 0..n {
        if i > 0 {
            packages.push(',');
        }
        packages.push_str(&format!(
            r#""pkg-{i}@1.0.{i}":{{"resolved":"https://registry.npmjs.org/pkg-{i}/-/pkg-{i}-1.0.{i}.tgz","integrity":"sha512-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==","dependencies":{{}}}}"#,
        ));
    }
    format!(r#"{{"lockfileVersion":1,"generatedBy":"guroku 1.0.0","packages":{{{packages}}}}}"#)
}

fn bench_parse(c: &mut Criterion) {
    for &n in &[1usize, 50, 500] {
        let body = build_lockfile_json(n);
        c.bench_function(&format!("lockfile_parse/{n}_packages"), |b| {
            b.iter(|| {
                let _: guroku::lockfile::Lockfile =
                    serde_json::from_slice(black_box(body.as_bytes())).unwrap();
            });
        });
    }
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);

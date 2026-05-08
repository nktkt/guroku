use criterion::{black_box, criterion_group, criterion_main, Criterion};

const MINIMAL: &str = r#"{"name":"x","version":"1.0.0"}"#;

const MEDIUM: &str = r#"{
  "name":"medium","version":"1.0.0",
  "dependencies":{"lodash":"^4","ms":"^2","chalk":"^5","debug":"^4","minimist":"^1","is-odd":"^3","is-number":"^7","ansi-styles":"^6","supports-color":"^9","yargs-parser":"^21"},
  "devDependencies":{"vitest":"^1"}
}"#;

const FULL: &str = r#"{
  "name":"full","version":"1.0.0",
  "description":"Big fixture","license":"MIT","private":true,
  "main":"index.js","keywords":["a","b","c"],
  "scripts":{"build":"tsc","test":"vitest","preinstall":"echo pre","postinstall":"echo post"},
  "bin":{"my-cli":"./bin/my-cli.js"},
  "workspaces":["packages/*"],
  "dependencies":{"lodash":"^4","ms":"^2"},
  "devDependencies":{"vitest":"^1","typescript":"^5"},
  "peerDependencies":{"react":"^18"},
  "optionalDependencies":{"fsevents":"^2"},
  "overrides":{"ms":"2.1.3"},
  "resolutions":{"left-pad":"1.3.0"},
  "homepage":"https://example.com",
  "repository":"github:example/full",
  "author":"Jane Doe <jane@example.com>",
  "engines":{"node":">=18"},
  "x-custom-1":"hello",
  "x-custom-2":[1,2,3]
}"#;

fn bench_manifest(c: &mut Criterion) {
    for (label, body) in &[("minimal", MINIMAL), ("medium", MEDIUM), ("full", FULL)] {
        c.bench_function(&format!("manifest_parse/{label}"), |b| {
            b.iter(|| {
                let _: guroku::manifest::Manifest =
                    serde_json::from_slice(black_box(body.as_bytes())).unwrap();
            });
        });
    }
}

criterion_group!(benches, bench_manifest);
criterion_main!(benches);

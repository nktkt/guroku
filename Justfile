# Justfile for the guroku project - common dev tasks
set shell := ["bash", "-uec"]

default:
    @just --list

build:
    cargo build

release:
    cargo build --release

test:
    cargo test --all

test-one TEST:
    cargo test --test {{TEST}}

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

check: fmt-check clippy test

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --open

run *ARGS:
    cargo run -- {{ARGS}}

clean:
    cargo clean

loc:
    find src tests -name '*.rs' | xargs wc -l | tail -1

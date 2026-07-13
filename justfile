default:
    @just --list

build:
    cargo build

release:
    cargo build --release

check:
    cargo fmt --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test

fmt:
    cargo fmt

run *args:
    cargo run -- {{args}}

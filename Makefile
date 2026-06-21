.PHONY: all build check run wasm test build_release test_release run_release

all: build build_release

build: check
	cargo build

check:
	cargo wgsl
	cargo check
	cargo clippy

test: build
	cargo test

run: build
	cargo run

build_release: check
	cargo build --release

test_release: build_release
	cargo test --release

run_release:
	cargo run -r

wasm:
	wasm-pack build --target web

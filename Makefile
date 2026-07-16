.PHONY: all build check run wasm test build_release test_release run_release build_release_opt test_release_opt test_release_all test_release_opt_all

all: build build_release

build: check
	cargo build

check:
	cargo wgsl
	cargo check
	cargo clippy

test: build
	mkdir -p test_outputs
	cargo test

run: build
	cargo run

build_release: check
	cargo build --release

test_release: build_release
	mkdir -p test_outputs
	cargo test --release

test_release_all: build_release
	mkdir -p test_outputs
	cargo test --release --features slow-tests

run_release:
	cargo run -r

build_release_opt: check
	cargo build --profile release-opt

test_release_opt: build_release_opt
	mkdir -p test_outputs
	cargo test --profile release-opt

test_release_opt_all: build_release_opt
	mkdir -p test_outputs
	cargo test --profile release-opt --features slow-tests

wasm:
	wasm-pack build --target web

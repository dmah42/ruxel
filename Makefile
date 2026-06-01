.PHONY: all build check run wasm

all: build build_release

build: check
	cargo build

check:
	cargo wgsl
	cargo check
	cargo clippy

run: build
	cargo run

build_release: check
	cargo build --release

run_release: build_release
	cargo run -r

wasm:
	wasm-pack build --target web

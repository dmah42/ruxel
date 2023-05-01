.PHONY: build check run wasm

build: check
	cargo build

check:
	cargo wgsl
	cargo check
	cargo clippy

run: build
	cargo run

wasm:
	wasm-pack build --target web

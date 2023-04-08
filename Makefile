.PHONY: build run wasm

build:
	cargo build

check:
	cargo check

run:
	cargo run

wasm:
	wasm-pack build --target web

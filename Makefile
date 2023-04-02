.PHONY: build run wasm

build:
	cargo build

run:
	cargo run

wasm:
	wasm-pack build --target web

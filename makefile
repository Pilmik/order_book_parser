.PHONY: all run test fmt lint check credits

all: test check

run:
	cargo run -- --help

test:
	cargo test

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

check: fmt lint test

credits:
	cargo run -- credits
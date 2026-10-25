CARGO_HOME := $(CURDIR)/.cache/cargo
export CARGO_HOME
.PHONY: test build dev test-core test-ui check

test: test-core test-ui

test-core:
	cargo test --manifest-path src-tauri/Cargo.toml --no-default-features

test-ui:
	npm test

check:
	cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
	npm run build

build:
	npm run build
	cargo build --manifest-path src-tauri/Cargo.toml

dev:
	npm run desktop

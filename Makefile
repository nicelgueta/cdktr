book:
	mdbook serve ./cdkr-book --open
test-main:
	cargo test
test-tui:
	cd tui && cargo test
test: test-main test-tui
fmt: 
	cargo fmt
build:
	cargo build
setup:
	cargo install mdbook-mermaid
commit:
	cargo fmt --check
push: test
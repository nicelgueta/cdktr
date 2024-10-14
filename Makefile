book:
	mdbook serve ./cdkr-book --open
test:
	cargo test
fmt: 
	cargo fmt
build:
	cargo build
setup:
	cargo install mdbook-mermaid
commit:
	cargo fmt --check
push: test
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
push: fmt test
book:
	mdbook serve ./cdkr-book --open
test:
	cargo test
build:
	cargo build
setup:
	cargo install mdbook-mermaid
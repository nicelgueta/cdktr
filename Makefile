book:
	mdbook serve ./cdkr-book --open
test:
	cargo test
setup:
	cargo install mdbook-mermaid
commit:
	cargo fmt --check
push: test
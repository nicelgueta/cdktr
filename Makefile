book:
	mdbook serve ./cdkr-book --open
test:
	cargo test
setup:
	cargo install mdbook-mermaid
commit:
	cargo fmt --check
push: test

run-principal:
	cargo run --bin cdktr-cli PRINCIPAL

run-agent:
	cargo run --bin cdktr-cli AGENT 5562
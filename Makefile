book:
	mdbook serve ./cdkr-book --open
test:
	cd cdktr && cargo test
setup:
# setup dotenv
	printf "CDKTR_PRINCIPAL_PORT=5561" > .env
# render mermaid diagrams in docs
	cargo install mdbook-mermaid
commit:
	cd cdktr && cargo fmt --check
push: test

principal:
	cd cdktr && cargo run --bin cdktr-cli PRINCIPAL

agent:
	cd cdktr && cargo run --bin cdktr-cli AGENT 5562

tui:
	cd cdktr && cargo run --bin cdktr-tui
	
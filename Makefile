book:
	mdbook serve ./cdkr-book --open
test:
	cd cdktr && cargo test
fmt:
	cd cdktr && cargo fmt
setup:
# setup dotenv
	printf "CDKTR_PRINCIPAL_PORT=5561\nRUST_LOG=info" > .env
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

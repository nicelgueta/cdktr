book:
	mdbook serve ./cdkr-book --open
test:
	cargo test
fmt:
	cargo fmt
setup:
# setup dotenv
	printf "CDKTR_PRINCIPAL_PORT=5561\nRUST_LOG=info" > .env
# render mermaid diagrams in docs
	cargo install mdbook-mermaid
commit:
	git diff --name-only --cached | grep '.rs$$' | xargs -n 1 rustfmt --edition 2024
	git add $(shell git diff --name-only --cached | grep '.rs$$')
push: test bump
bump:
	bash scripts/bump_versions.sh
release:
	cargo build --release
principal:
	cargo run start principal
agent:
	cargo run start agent
ui:
	cargo run ui
pycli:
	cd python-cdktr && uv run cli_zmq.py
run:
	./target/release/cdktr
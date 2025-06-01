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
	git diff --name-only --cached | grep '.rs$$' | xargs -n 1 rustfmt --edition 2021
	git add $(shell git diff --name-only --cached | grep '.rs$$')
push: test

principal:
	cargo run start -i principal
agent:
	cargo run start -i agent
tui:
	cargo run ui
pycli:
	python python-cdktr/cli_zmq.py
run-migration:
	diesel migration run
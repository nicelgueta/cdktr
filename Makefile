book:
	mdbook serve ./cdkr-book --open
build-book:
	mdbook build ./cdkr-book -d ../../nicelgueta.github.io/cdktr
test:
	cargo test
fmt:
	cargo fmt
setup:
# setup dotenv
	printf "CDKTR_PRINCIPAL_PORT=5561\nRUST_LOG=info" > .env
# render mermaid diagrams in docs
	cargo install mdbook-mermaid
# setup python venv and install maturin for building python library
	cd pycdktr && python3 -m venv venv && . venv/bin/activate && pip install maturin
commit:
	git diff --name-only --cached | grep '.rs$$' | xargs -n 1 rustfmt --edition 2024
	git add $(shell git diff --name-only --cached | grep '.rs$$')
push: test
release:
	bash scripts/release.sh
build-release:
	cargo build --release
principal:
	cargo run start principal
principal-no-scheduler:
	cargo run start principal --no-scheduler
agent:
	cargo run start agent
ui:
	cargo run ui
run:
	./target/release/cdktr
sync-main:
	git checkout main
	git pull origin main
	git checkout develop
	git merge main

# Python library build targets
build-py:
	cd pycdktr && . venv/bin/activate && maturin develop

build-py-wheel:
	cd pycdktr && . venv/bin/activate && maturin build --release
# disco — developer tasks. The installed command is `disko`.
.PHONY: help build release run test fmt lint check install clean fixture

help: ## Show this help
	@grep -E '^[a-z-]+:.*##' $(MAKEFILE_LIST) | awk -F':.*##' '{printf "  \033[36m%-9s\033[0m %s\n", $$1, $$2}'

build: ## Debug build
	cargo build

release: ## Optimized release build
	cargo build --release

run: ## Run; pass args via ARGS, e.g. make run ARGS="scan ~/code"
	cargo run -- $(ARGS)

test: ## Run the test suite
	cargo test

fmt: ## Format the code
	cargo fmt

lint: ## Clippy with warnings denied
	cargo clippy --all-targets -- -D warnings

check: ## Done-bar gate: format-check, lint, test (makes no changes)
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test

fixture: ## Create a sample tree of artifacts and print its path (try: disko "$$(make -s fixture)")
	@scripts/fixture.sh

install: ## Install the `disko` command into ~/.cargo/bin
	cargo install --path .

clean: ## Remove build artifacts
	cargo clean

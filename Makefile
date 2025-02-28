.PHONY: help fmt lint test build clean check doc install install-dev-tools check-license update-license all perf-test perf-test-add perf-test-update perf-test-check perf-test-all docker-build docker-build-debug docker-build-distroless docker-run docker-run-debug docker-run-distroless docker-clean

# Default target

### Makefile

help: ## Display this help
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@awk 'BEGIN {section="General"} /^### /{section=substr($$0,5); printf "\n\033[1m%s\033[0m\n", section} /^[a-zA-Z0-9_-]+:.*?## / {match($$0, /## (.*)$$/, a); printf "  \033[36m%-18s\033[0m %s\n", substr($$1,1,length($$1)-1), a[1]}' $(MAKEFILE_LIST)

### Development
fmt: ## Format code using rustfmt
	cargo fmt --all

lint: ## Run clippy for linting
	cargo clippy -- -D warnings

lint-all: ## Run clippy with all features
	cargo clippy --all-features -- -D warnings

test: build ## Run tests
	cargo test

test-all: ## Run tests with all features
	cargo test --all-features

check: ## Run cargo check
	cargo check

doc: ## Generate documentation
	cargo doc --no-deps

watch-test: ## Run tests in watch mode (requires cargo-watch)
	cargo watch -x test

all: fmt lint test docker-build ## Run verify-config, fmt, lint, and test

### Build
build: ## Build the project
	cargo build

release: ## Build release version
	cargo build --release

clean: ## Clean build artifacts
	cargo clean

run: ## Run the application
	cargo run

### Installation
install: ## Install edlicense locally
	cargo install --path .

install-dev-tools: ## Install development tools
	rustup show # Ensures rust-toolchain.toml is applied
	cargo install cargo-watch
	cargo install cargo-outdated

### Docker
docker-build: ## Build lightweight Docker image (Debian-based)
	docker build -t edlicense:latest --build-arg MODE=production .

docker-build-distroless: ## Build minimal distroless Docker image
	docker build -t edlicense:distroless --build-arg MODE=distroless .

docker-build-debug: ## Build debug/development Docker image
	docker build -t edlicense:debug --build-arg MODE=debug .

docker-build-all: docker-build docker-build-distroless docker-build-debug ## Build all Docker images

docker-run: ## Run Docker container with current directory mounted
	docker run --rm -v "$(shell pwd):/workspace" -w /workspace edlicense:latest $(ARGS)

docker-run-distroless: ## Run distroless Docker container with current directory mounted
	docker run --rm -v "$(shell pwd):/workspace" -w /workspace edlicense:distroless $(ARGS)

docker-run-debug: ## Run debug Docker container with current directory mounted
	docker run --rm -it -v "$(shell pwd):/usr/src/edlicense" edlicense:debug $(ARGS)

docker-clean: ## Remove Docker images
	docker rmi -f edlicense:latest edlicense:distroless edlicense:debug 2>/dev/null || true

### Performance Testing
perf-test-add: build ## Run performance test for adding licenses to files
	cargo test --release test_add_license_performance -- --ignored --nocapture

perf-test-update: build ## Run performance test for updating license years
	cargo test --release test_update_year_performance -- --ignored --nocapture

perf-test-check: build ## Run performance test for checking license headers (dry run mode)
	cargo test --release test_check_license_performance -- --ignored --nocapture

perf-test-file-size: build ## Run performance test with different file sizes
	cargo test --release test_file_size_impact -- --ignored --nocapture

perf-test-threads: build ## Run performance test with different thread counts
	cargo test --release test_thread_count_impact -- --ignored --nocapture

perf-benchmark: build ## Run comprehensive benchmark tests
	cargo test --release benchmark_operations -- --ignored --nocapture

perf-test-all: build ## Run all performance tests
	@echo "Running all performance tests (this may take a while)..."
	cargo test --release -- --ignored --nocapture
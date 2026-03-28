SHELL := /bin/zsh

CARGO := cargo
PYTHON := python3
BIN := target/release/edgesvg
GOLDEN_DIR := golden_data
SAMPLE_LIMIT := 90
SMOKE_LIMIT := 12
OODA_LOOPS := 50

.DEFAULT_GOAL := help

.PHONY: help fmt test build verify verify-all python-sdk node-sdk wasm-sdk bench bench-smoke bench-sample bench-full optimize-frontier optimize-ooda clean-bench

help:
	@printf "\nEdgeSVG Workflow\n\n"
	@printf "  %-20s %s\n" "make verify" "fmt + test"
	@printf "  %-20s %s\n" "make verify-all" "Rust + Python + Node + WASM checks"
	@printf "  %-20s %s\n" "make build" "Build release binary"
	@printf "  %-20s %s\n" "make python-sdk" "Build and test the Python package"
	@printf "  %-20s %s\n" "make node-sdk" "Build and test the Node.js package"
	@printf "  %-20s %s\n" "make wasm-sdk" "Check the WASM crate"
	@printf "  %-20s %s\n" "make bench" "Alias for make bench-sample"
	@printf "  %-20s %s\n" "make bench-smoke" "Fast 12-asset golden benchmark"
	@printf "  %-20s %s\n" "make bench-sample" "Main 90-asset benchmark with diff"
	@printf "  %-20s %s\n" "make bench-full" "Full golden-data verification"
	@printf "  %-20s %s\n" "make optimize-frontier" "10-loop OODA optimization sweep"
	@printf "  %-20s %s\n\n" "make clean-bench" "Remove generated benchmark artifacts"

fmt:
	$(CARGO) fmt --all

test:
	$(CARGO) test

verify: fmt test

verify-all: verify python-sdk node-sdk wasm-sdk

build:
	$(CARGO) build --release

python-sdk:
	cd sdks/python && maturin develop && pytest -q

node-sdk:
	cd sdks/node && npm ci && npm run build && npm test

wasm-sdk:
	$(CARGO) check -p edgesvg-wasm --target wasm32-unknown-unknown

bench: bench-sample

bench-smoke: build
	$(PYTHON) scripts/benchmark_suite.py \
		--suite smoke \
		--bin $(BIN) \
		--golden-dir $(GOLDEN_DIR)

bench-sample: build
	$(PYTHON) scripts/benchmark_suite.py \
		--suite sample \
		--bin $(BIN) \
		--golden-dir $(GOLDEN_DIR) \
		--baseline-json benchmark_runs/golden_full/report.json \
		--limit $(SAMPLE_LIMIT)

bench-full: build
	$(PYTHON) scripts/benchmark_suite.py \
		--suite full \
		--bin $(BIN) \
		--golden-dir $(GOLDEN_DIR)

optimize-frontier: build
	$(PYTHON) scripts/optimize_frontier.py \
		--bin $(BIN) \
		--golden-dir $(GOLDEN_DIR) \
		--limit $(SAMPLE_LIMIT) \
		--loops $(OODA_LOOPS)

optimize-ooda: optimize-frontier

clean-bench:
	rm -rf benchmark_runs/golden_smoke \
		benchmark_runs/golden_sample \
		benchmark_runs/golden_full \
		benchmark_runs/optimization_frontier

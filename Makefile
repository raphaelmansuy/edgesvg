SHELL := /bin/zsh

CARGO := cargo
PYTHON := python3
BIN := target/release/edgesvg
GOLDEN_DIR := golden_data
SAMPLE_LIMIT := 90
SMOKE_LIMIT := 12
OODA_LOOPS := 10

.DEFAULT_GOAL := help

.PHONY: help fmt test build verify bench bench-smoke bench-sample bench-full optimize-frontier optimize-ooda clean-bench

help:
	@printf "\nEdgeSVG Workflow\n\n"
	@printf "  %-20s %s\n" "make verify" "fmt + test"
	@printf "  %-20s %s\n" "make build" "Build release binary"
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

build:
	$(CARGO) build --release

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
		benchmark_runs/golden_full_current \
		benchmark_runs/optimization_frontier

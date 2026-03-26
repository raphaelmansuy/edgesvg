SHELL := /bin/zsh

CARGO := cargo
PYTHON := python3
BIN := target/release/edgesvg
GOLDEN_DIR := golden_data
SAMPLE_LIMIT := 90
SMOKE_LIMIT := 12

.PHONY: help fmt test build bench-smoke bench-sample bench-full optimize-frontier clean-bench

help:
	@printf "\nEdgeSVG Workflow\n\n"
	@printf "  %-18s %s\n" "make build" "Build release binary"
	@printf "  %-18s %s\n" "make fmt" "Format Rust sources"
	@printf "  %-18s %s\n" "make test" "Run full test suite"
	@printf "  %-18s %s\n" "make bench-smoke" "Fast golden-data smoke benchmark"
	@printf "  %-18s %s\n" "make bench-sample" "90-asset benchmark with baseline diff"
	@printf "  %-18s %s\n" "make bench-full" "Full golden-data benchmark"
	@printf "  %-18s %s\n" "make optimize-frontier" "Run bounded optimization sweep"
	@printf "  %-18s %s\n\n" "make clean-bench" "Remove generated benchmark artifacts"

fmt:
	$(CARGO) fmt --all

test:
	$(CARGO) test

build:
	$(CARGO) build --release

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
		--limit $(SAMPLE_LIMIT)

clean-bench:
	rm -rf benchmark_runs/golden_smoke \
		benchmark_runs/golden_sample \
		benchmark_runs/golden_full_current \
		benchmark_runs/optimization_frontier

#!/usr/bin/env python3

import argparse
import json
import subprocess
from pathlib import Path


SUITES = {
    "smoke": {
        "limit": 12,
        "quality": "figma",
        "max_iterations": 2,
        "target_ssim": 0.998,
        "max_file_size": 100_000,
        "work_dir": "benchmark_runs/golden_smoke",
    },
    "sample": {
        "limit": 90,
        "quality": "figma",
        "max_iterations": 4,
        "target_ssim": 0.998,
        "max_file_size": 100_000,
        "work_dir": "benchmark_runs/golden_sample",
    },
    "full": {
        "limit": None,
        "quality": "figma",
        "max_iterations": 4,
        "target_ssim": 0.998,
        "max_file_size": 100_000,
        "work_dir": "benchmark_runs/golden_full_current",
    },
}


def run(cmd, cwd: Path) -> None:
    subprocess.run(cmd, cwd=cwd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def load_json(path: Path) -> dict:
    return json.loads(path.read_text())


def print_report(report: dict) -> None:
    print("\nOverall")
    print(
        "entries={entries} ssim={ssim:.4f} psnr={psnr:.2f} mae={mae:.2f} "
        "size={size:.1f}KB paths={paths:.1f} edge={edge:.4f} topo={topo:.4f} "
        "time={time:.1f}ms ips={ips:.2f}".format(
            entries=len(report["entries"]),
            ssim=report["average_ssim"],
            psnr=report["average_psnr"],
            mae=report["average_mae"],
            size=report["average_file_size"] / 1024.0,
            paths=report["average_path_count"],
            edge=report["average_edge_similarity"],
            topo=report["average_topology_score"],
            time=report["average_elapsed_ms"],
            ips=report["throughput_images_per_sec"],
        )
    )

    print("\nBy Group")
    for group in report["groups"]:
        print(
            "{group:14s} entries={entries:3d} ssim={ssim:.4f} size={size:.1f}KB "
            "paths={paths:.1f} time={time:.1f}ms".format(
                group=group["group"],
                entries=group["entries"],
                ssim=group["average_ssim"],
                size=group["average_file_size"] / 1024.0,
                paths=group["average_path_count"],
                time=group["average_elapsed_ms"],
            )
        )


def print_delta(current: dict, baseline: dict) -> None:
    print("\nDelta vs Baseline")
    metrics = [
        ("average_ssim", "ssim", False),
        ("average_psnr", "psnr", False),
        ("average_mae", "mae", True),
        ("average_file_size", "size_bytes", True),
        ("average_path_count", "path_count", True),
        ("average_elapsed_ms", "elapsed_ms", True),
    ]
    for key, label, lower_is_better in metrics:
        delta = current[key] - baseline[key]
        better = delta < 0 if lower_is_better else delta > 0
        verdict = "better" if better else "worse"
        print(f"{label:12s} delta={delta:+.4f} {verdict}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run a reproducible EdgeSVG benchmark suite with stable artifacts."
    )
    parser.add_argument("--suite", choices=sorted(SUITES), default="sample")
    parser.add_argument("--golden-dir", default="golden_data")
    parser.add_argument("--bin", default="target/release/edgesvg")
    parser.add_argument("--baseline-json")
    parser.add_argument("--quality")
    parser.add_argument("--max-iterations", type=int)
    parser.add_argument("--target-ssim", type=float)
    parser.add_argument("--max-file-size", type=int)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--work-dir")
    args = parser.parse_args()

    root = Path.cwd()
    cfg = dict(SUITES[args.suite])
    if args.quality is not None:
        cfg["quality"] = args.quality
    if args.max_iterations is not None:
        cfg["max_iterations"] = args.max_iterations
    if args.target_ssim is not None:
        cfg["target_ssim"] = args.target_ssim
    if args.max_file_size is not None:
        cfg["max_file_size"] = args.max_file_size
    if args.limit is not None:
        cfg["limit"] = args.limit
    if args.work_dir is not None:
        cfg["work_dir"] = args.work_dir

    bin_path = root / args.bin
    work_dir = root / cfg["work_dir"]
    json_path = work_dir / "report.json"
    markdown_path = work_dir / "report.md"
    work_dir.mkdir(parents=True, exist_ok=True)

    cmd = [
        str(bin_path),
        "benchmark-golden",
        "--golden-dir",
        args.golden_dir,
        "--work-dir",
        str(work_dir),
        "--quality",
        cfg["quality"],
        "--target-ssim",
        str(cfg["target_ssim"]),
        "--max-file-size",
        str(cfg["max_file_size"]),
        "--max-iterations",
        str(cfg["max_iterations"]),
        "--json-path",
        str(json_path),
        "--markdown-path",
        str(markdown_path),
    ]
    if cfg["limit"] is not None:
        cmd.extend(["--limit", str(cfg["limit"])])

    print("Running suite")
    print(
        "suite={suite} quality={quality} iterations={iterations} target_ssim={target_ssim} "
        "max_file_size={max_file_size} limit={limit}".format(
            suite=args.suite,
            quality=cfg["quality"],
            iterations=cfg["max_iterations"],
            target_ssim=cfg["target_ssim"],
            max_file_size=cfg["max_file_size"],
            limit=cfg["limit"],
        )
    )
    run(cmd, root)

    report = load_json(json_path)
    print_report(report)

    if args.baseline_json:
        baseline = load_json(root / args.baseline_json)
        print_delta(report, baseline)

    print(f"\nArtifacts\njson={json_path}\nmarkdown={markdown_path}")


if __name__ == "__main__":
    main()

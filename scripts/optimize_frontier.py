#!/usr/bin/env python3

import argparse
import json
import subprocess
from pathlib import Path


DEFAULT_CONFIGS = [
    {"quality": "figma", "max_iterations": 2},
    {"quality": "figma", "max_iterations": 3},
    {"quality": "figma", "max_iterations": 4},
    {"quality": "balanced", "max_iterations": 2},
    {"quality": "balanced", "max_iterations": 3},
    {"quality": "quality", "max_iterations": 2},
    {"quality": "ultra", "max_iterations": 1},
    {"quality": "figma", "max_iterations": 4, "target_ssim": 0.995},
    {"quality": "balanced", "max_iterations": 3, "target_ssim": 0.995},
    {"quality": "quality", "max_iterations": 2, "target_ssim": 0.995},
]


def score(report: dict) -> float:
    return (
        report["average_ssim"] * 0.75
        + max(0.0, 1.0 - report["average_file_size"] / 20_000.0) * 0.15
        + max(0.0, 1.0 - report["average_path_count"] / 200.0) * 0.10
    )


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run a bounded optimization frontier sweep on the golden corpus."
    )
    parser.add_argument("--bin", default="target/release/edgesvg")
    parser.add_argument("--golden-dir", default="golden_data")
    parser.add_argument("--limit", type=int, default=90)
    parser.add_argument("--output-dir", default="benchmark_runs/optimization_frontier")
    args = parser.parse_args()

    root = Path.cwd()
    bin_path = root / args.bin
    output_dir = root / args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)

    results = []
    for index, cfg in enumerate(DEFAULT_CONFIGS, 1):
        trial_dir = output_dir / f"trial_{index:02d}_{cfg['quality']}_{cfg['max_iterations']}"
        json_path = trial_dir / "report.json"
        cmd = [
            str(bin_path),
            "benchmark-golden",
            "--golden-dir",
            args.golden_dir,
            "--work-dir",
            str(trial_dir),
            "--quality",
            cfg["quality"],
            "--max-iterations",
            str(cfg["max_iterations"]),
            "--target-ssim",
            str(cfg.get("target_ssim", 0.998)),
            "--json-path",
            str(json_path),
            "--limit",
            str(args.limit),
        ]
        print(f"[{index}/{len(DEFAULT_CONFIGS)}] {cfg}", flush=True)
        subprocess.run(cmd, cwd=root, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        report = json.loads(json_path.read_text())
        result = {
            "index": index,
            "config": cfg,
            "average_ssim": report["average_ssim"],
            "average_psnr": report["average_psnr"],
            "average_mae": report["average_mae"],
            "average_file_size": report["average_file_size"],
            "average_path_count": report["average_path_count"],
            "average_elapsed_ms": report["average_elapsed_ms"],
        }
        result["score"] = score(result)
        results.append(result)
        print(
            "score={score:.4f} ssim={ssim:.4f} size={size:.1f}KB paths={paths:.1f} time={time:.1f}ms".format(
                score=result["score"],
                ssim=result["average_ssim"],
                size=result["average_file_size"] / 1024.0,
                paths=result["average_path_count"],
                time=result["average_elapsed_ms"],
            ),
            flush=True,
        )

    ranked = sorted(results, key=lambda item: item["score"], reverse=True)
    summary_path = output_dir / "summary.json"
    summary_path.write_text(json.dumps(ranked, indent=2))

    print("\nTop Configs")
    for item in ranked[:5]:
        print(
            "{index:02d} {config} score={score:.4f} ssim={ssim:.4f} size={size:.1f}KB paths={paths:.1f}".format(
                index=item["index"],
                config=item["config"],
                score=item["score"],
                ssim=item["average_ssim"],
                size=item["average_file_size"] / 1024.0,
                paths=item["average_path_count"],
            )
        )
    print(f"\nSummary\n{summary_path}")


if __name__ == "__main__":
    main()

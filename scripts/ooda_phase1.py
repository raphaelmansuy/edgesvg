#!/usr/bin/env python3

import argparse
import json
import statistics
import subprocess
from pathlib import Path


def build_configs():
    configs = []
    for target in [0.90, 0.92, 0.95, 0.98]:
        for size in [20, 50, 100, 200]:
            configs.append(
                {"kind": "smart", "target": target, "size": size, "iterations": 5}
            )
    for target in [0.95, 0.98]:
        for colors in [None, 8, 16, 24, 32, 48, 64, 96]:
            configs.append({"kind": "premium", "target": target, "colors": colors})
    for quality in ["clean", "balanced", "high", "ultra"]:
        for colors in [None, 8, 16]:
            configs.append({"kind": "logo", "quality": quality, "colors": colors})
    for quality in ["figma", "balanced", "quality", "ultra"]:
        configs.append({"kind": "hifi", "quality": quality})
    configs.append({"kind": "auto"})
    configs.append({"kind": "optimal"})
    return configs


def build_command(bin_path: Path, work_dir: Path, asset: str, cfg: dict, trial_idx: int):
    output = work_dir / f"trial_{trial_idx}.svg"
    if cfg["kind"] == "smart":
        return [
            str(bin_path),
            "smart",
            asset,
            str(output),
            "--json",
            "--quality",
            str(cfg["target"]),
            "--size",
            str(cfg["size"]),
            "--iterations",
            str(cfg["iterations"]),
        ]
    if cfg["kind"] == "premium":
        command = [
            str(bin_path),
            "premium",
            asset,
            str(output),
            "--json",
            "--target",
            str(cfg["target"]),
        ]
        if cfg["colors"] is not None:
            command += ["--colors", str(cfg["colors"])]
        return command
    if cfg["kind"] == "logo":
        command = [
            str(bin_path),
            "logo",
            asset,
            str(output),
            "--json",
            "--quality",
            cfg["quality"],
        ]
        if cfg["colors"] is not None:
            command += ["--colors", str(cfg["colors"])]
        return command
    if cfg["kind"] == "hifi":
        return [
            str(bin_path),
            "convert",
            asset,
            str(output),
            "--json",
            "--method",
            "hifi",
            "--quality",
            cfg["quality"],
        ]
    if cfg["kind"] == "auto":
        return [str(bin_path), "auto", asset, str(output), "--json"]
    if cfg["kind"] == "optimal":
        return [str(bin_path), "convert", asset, str(output), "--json", "--method", "optimal"]
    raise ValueError(f"unsupported config: {cfg}")


def score_trial(metrics):
    avg_size = metrics["avg_file_size"]
    avg_paths = metrics["avg_path_count"]
    return (
        metrics["avg_ssim"] * 0.55
        + metrics["avg_edge_similarity"] * 0.2
        + metrics["avg_topology_score"] * 0.1
        + max(0.0, 1.0 - avg_size / 120_000.0) * 0.1
        + max(0.0, 1.0 - avg_paths / 800.0) * 0.05
    )


def summarize_assets(entries):
    metrics = {
        "avg_ssim": statistics.mean(entry["ssim"] for entry in entries),
        "avg_psnr": statistics.mean(entry["psnr"] for entry in entries),
        "avg_mae": statistics.mean(entry["mae"] for entry in entries),
        "avg_file_size": statistics.mean(entry["file_size"] for entry in entries),
        "avg_path_count": statistics.mean(entry["path_count"] for entry in entries),
        "avg_edge_similarity": statistics.mean(
            entry["edge_similarity"] for entry in entries
        ),
        "avg_topology_score": statistics.mean(
            entry["topology_score"] for entry in entries
        ),
    }
    metrics["score"] = score_trial(metrics)
    return metrics


def run_trial(root: Path, bin_path: Path, work_dir: Path, assets, cfg, idx: int):
    per_asset = []
    ok = True
    for asset in assets:
        command = build_command(bin_path, work_dir, asset, cfg, idx)
        try:
            result = subprocess.run(
                command, cwd=root, capture_output=True, text=True, timeout=20
            )
        except subprocess.TimeoutExpired:
            ok = False
            per_asset.append({"asset": asset, "error": "timeout"})
            continue

        if result.returncode != 0:
            ok = False
            per_asset.append({"asset": asset, "error": result.stderr[-400:]})
            continue

        try:
            payload = json.loads(result.stdout)
        except json.JSONDecodeError:
            ok = False
            per_asset.append({"asset": asset, "error": result.stdout[-400:]})
            continue

        report = payload.get("report", payload)
        metrics = report["metrics"]
        per_asset.append(
            {
                "asset": asset,
                "ssim": metrics["ssim"],
                "psnr": metrics["psnr"],
                "mae": metrics["mae"],
                "file_size": metrics["file_size"],
                "path_count": metrics["path_count"],
                "topology_score": metrics.get("topology_score", 0.0),
                "edge_similarity": metrics.get("edge_similarity", 0.0),
            }
        )

    scored = [entry for entry in per_asset if "ssim" in entry]
    summary = summarize_assets(scored) if scored else summarize_assets([{
        "ssim": 0.0,
        "psnr": 0.0,
        "mae": 255.0,
        "file_size": 1_000_000,
        "path_count": 10_000,
        "topology_score": 0.0,
        "edge_similarity": 0.0,
    }])
    return {
        "index": idx,
        "config": cfg,
        "ok": ok,
        **summary,
        "per_asset": per_asset,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output-dir",
        default="benchmark_runs/ooda_trials",
        help="directory for experiment artifacts",
    )
    parser.add_argument(
        "--bin",
        default="target/debug/edgesvg",
        help="path to built edgesvg binary",
    )
    args = parser.parse_args()

    root = Path.cwd()
    output_dir = root / args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)
    bin_path = root / args.bin

    assets = [
        "benchmark_runs/golden_full/rendered_inputs/icons/phone-off.png",
        "benchmark_runs/golden_full/rendered_inputs/icons/edit.png",
        "benchmark_runs/golden_full/rendered_inputs/logos/simple_elasticstack.png",
        "benchmark_runs/golden_full/rendered_inputs/logos/simple_fiat.png",
        "benchmark_runs/golden_full/rendered_inputs/illustrations/twemoji_1f38b.png",
        "benchmark_runs/golden_full/rendered_inputs/illustrations/twemoji_1f326.png",
    ]
    configs = build_configs()

    results = []
    for idx, cfg in enumerate(configs, 1):
        result = run_trial(root, bin_path, output_dir, assets, cfg, idx)
        results.append(result)
        print(
            "[{}/{}] {} -> score={:.4f} ssim={:.4f} size={:.1f}KB paths={:.1f}".format(
                str(idx).zfill(2),
                len(configs),
                cfg,
                result["score"],
                result["avg_ssim"],
                result["avg_file_size"] / 1024.0,
                result["avg_path_count"],
            ),
            flush=True,
        )

    (output_dir / "phase1_results.json").write_text(json.dumps(results, indent=2))
    ranked = sorted(results, key=lambda item: item["score"], reverse=True)
    summary = {
        "assets": assets,
        "top10": [
            {
                "index": item["index"],
                "config": item["config"],
                "score": item["score"],
                "avg_ssim": item["avg_ssim"],
                "avg_file_size": item["avg_file_size"],
                "avg_path_count": item["avg_path_count"],
                "avg_edge_similarity": item["avg_edge_similarity"],
            }
            for item in ranked[:10]
        ],
    }
    (output_dir / "phase1_summary.json").write_text(json.dumps(summary, indent=2))

    print("\nTop 10")
    for item in ranked[:10]:
        print(
            "{} {} score={:.4f} ssim={:.4f} size={:.1f}KB paths={:.1f}".format(
                item["index"],
                item["config"],
                item["score"],
                item["avg_ssim"],
                item["avg_file_size"] / 1024.0,
                item["avg_path_count"],
            )
        )


if __name__ == "__main__":
    main()

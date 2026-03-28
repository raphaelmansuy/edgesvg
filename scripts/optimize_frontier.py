#!/usr/bin/env python3

import argparse
import json
import subprocess
from pathlib import Path


QUALITY_ORDER = ["figma", "balanced", "quality", "ultra"]
TARGET_SSIM_OPTIONS = [0.99, 0.995, 0.998]
MAX_ITER_OPTIONS = [1, 2, 3, 4]
MAX_FILE_SIZE_OPTIONS = [25_000, 50_000, 75_000, 100_000]
SEED_CONFIGS = [
    {"quality": "figma", "max_iterations": 3, "target_ssim": 0.998, "max_file_size": 100_000},
    {"quality": "figma", "max_iterations": 4, "target_ssim": 0.995, "max_file_size": 100_000},
    {"quality": "balanced", "max_iterations": 3, "target_ssim": 0.995, "max_file_size": 100_000},
]


def compactness_score(report: dict) -> float:
    return (
        max(0.0, 1.0 - report["average_file_size"] / 20_000.0) * 0.6
        + max(0.0, 1.0 - report["average_path_count"] / 200.0) * 0.3
        + max(0.0, 1.0 - report["average_elapsed_ms"] / 150.0) * 0.1
    )


def score(report: dict) -> float:
    return (
        report["average_fidelity_score"] * 0.74
        + report["average_edge_f1"] * 0.10
        + report["average_foreground_iou"] * 0.06
        + report["average_color_similarity"] * 0.04
        + compactness_score(report) * 0.06
    )


def config_key(cfg: dict) -> str:
    return json.dumps(cfg, sort_keys=True)


def clamp_quality(index: int) -> str:
    return QUALITY_ORDER[max(0, min(index, len(QUALITY_ORDER) - 1))]


def build_config_space() -> list[dict]:
    return [
        {
            "quality": quality,
            "max_iterations": max_iterations,
            "target_ssim": target_ssim,
            "max_file_size": max_file_size,
        }
        for quality in QUALITY_ORDER
        for max_iterations in MAX_ITER_OPTIONS
        for target_ssim in TARGET_SSIM_OPTIONS
        for max_file_size in MAX_FILE_SIZE_OPTIONS
    ]


def config_distance(left: dict, right: dict) -> tuple:
    return (
        abs(QUALITY_ORDER.index(left["quality"]) - QUALITY_ORDER.index(right["quality"])),
        abs(left["max_iterations"] - right["max_iterations"]),
        abs(left["target_ssim"] - right["target_ssim"]),
        abs(left["max_file_size"] - right["max_file_size"]),
    )


def next_unexplored_configs(best: dict, evaluated: dict, limit: int) -> list[dict]:
    ranked = sorted(
        build_config_space(),
        key=lambda candidate: config_distance(candidate, best),
    )
    queue = []
    for candidate in ranked:
        if config_key(candidate) in evaluated:
            continue
        queue.append(candidate)
        if len(queue) >= limit:
            break
    return queue


def mutate_config(base: dict, observation: dict, loop_index: int) -> list[dict]:
    quality_idx = QUALITY_ORDER.index(base["quality"])
    candidates = []

    fidelity = observation["average_fidelity_score"]
    edge_f1 = observation["average_edge_f1"]
    size = observation["average_file_size"]
    paths = observation["average_path_count"]

    if fidelity < 0.88 or edge_f1 < 0.86:
        candidates.append(
            {
                **base,
                "quality": clamp_quality(quality_idx + 1),
                "max_iterations": min(base["max_iterations"] + 1, 4),
                "target_ssim": max(base["target_ssim"], 0.998),
            }
        )
    if size > 3_000 or paths > 24:
        candidates.append(
            {
                **base,
                "quality": clamp_quality(quality_idx - 1),
                "max_iterations": max(base["max_iterations"] - 1, 2),
                "target_ssim": min(base["target_ssim"], 0.995),
                "max_file_size": min(base["max_file_size"], 75_000),
            }
        )

    candidates.extend(
        [
            {
                **base,
                "quality": clamp_quality(quality_idx + (1 if loop_index % 2 == 0 else -1)),
                "max_iterations": 4 if fidelity < 0.90 else 3,
            },
            {
                **base,
                "target_ssim": 0.995 if base["target_ssim"] >= 0.998 else 0.998,
                "max_iterations": min(max(base["max_iterations"], 3) + (loop_index % 2), 4),
            },
            {
                **base,
                "max_file_size": 50_000 if size < 2_500 else 100_000,
            },
        ]
    )

    deduped = []
    seen = set()
    for candidate in candidates:
        key = config_key(candidate)
        if key in seen:
            continue
        deduped.append(candidate)
        seen.add(key)
    return deduped[:4]


def observe(report: dict) -> dict:
    return {
        "average_fidelity_score": report["average_fidelity_score"],
        "average_ssim": report["average_ssim"],
        "average_edge_f1": report["average_edge_f1"],
        "average_foreground_iou": report["average_foreground_iou"],
        "average_color_similarity": report["average_color_similarity"],
        "average_file_size": report["average_file_size"],
        "average_path_count": report["average_path_count"],
        "average_elapsed_ms": report["average_elapsed_ms"],
    }


def orient(observation: dict) -> str:
    if observation["average_fidelity_score"] < 0.88:
        return "fidelity-limited"
    if observation["average_edge_f1"] < 0.86:
        return "edge-limited"
    if observation["average_file_size"] > 3_000 or observation["average_path_count"] > 24:
        return "compactness-limited"
    return "frontier-balance"


def run_trial(
    root: Path,
    bin_path: Path,
    golden_dir: str,
    output_dir: Path,
    cfg: dict,
    limit: int,
    index: int,
) -> dict:
    trial_dir = output_dir / f"trial_{index:02d}_{cfg['quality']}_{cfg['max_iterations']}"
    json_path = trial_dir / "report.json"
    cmd = [
        str(bin_path),
        "benchmark-golden",
        "--golden-dir",
        golden_dir,
        "--work-dir",
        str(trial_dir),
        "--quality",
        cfg["quality"],
        "--max-iterations",
        str(cfg["max_iterations"]),
        "--target-ssim",
        str(cfg["target_ssim"]),
        "--max-file-size",
        str(cfg["max_file_size"]),
        "--json-path",
        str(json_path),
        "--limit",
        str(limit),
    ]
    subprocess.run(cmd, cwd=root, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    report = json.loads(json_path.read_text())
    summary = observe(report)
    result = {"index": index, "config": cfg, **summary}
    result["score"] = score(result)
    result["trial_dir"] = str(trial_dir)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run a first-principles OODA optimization loop on the golden corpus."
    )
    parser.add_argument("--bin", default="target/release/edgesvg")
    parser.add_argument("--golden-dir", default="golden_data")
    parser.add_argument("--limit", type=int, default=90)
    parser.add_argument("--loops", type=int, default=50)
    parser.add_argument("--output-dir", default="benchmark_runs/optimization_frontier")
    args = parser.parse_args()

    root = Path.cwd()
    bin_path = root / args.bin
    output_dir = root / args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)

    evaluated = {}
    history = []
    queue = list(SEED_CONFIGS)
    trial_index = 0

    loop_index = 0
    while loop_index < args.loops and queue:
        cfg = queue.pop(0)
        key = config_key(cfg)
        if key in evaluated:
            continue

        loop_index += 1

        trial_index += 1
        print(f"\nLoop {loop_index:02d} observe", flush=True)
        print(f"config={cfg}", flush=True)
        result = run_trial(
            root,
            bin_path,
            args.golden_dir,
            output_dir,
            cfg,
            args.limit,
            trial_index,
        )
        evaluated[key] = result

        observation = observe(result)
        phenotype = orient(observation)
        decision = mutate_config(cfg, observation, loop_index)
        history.append(
            {
                "loop": loop_index,
                "observation": observation,
                "phenotype": phenotype,
                "config": cfg,
                "score": result["score"],
                "decision_candidates": decision,
                "trial_dir": result["trial_dir"],
            }
        )

        print(
            "orient phenotype={phenotype} score={score:.4f} fidelity={fidelity:.4f} "
            "edge_f1={edge_f1:.4f} size={size:.1f}KB paths={paths:.1f} time={time:.1f}ms".format(
                phenotype=phenotype,
                score=result["score"],
                fidelity=result["average_fidelity_score"],
                edge_f1=result["average_edge_f1"],
                size=result["average_file_size"] / 1024.0,
                paths=result["average_path_count"],
                time=result["average_elapsed_ms"],
            ),
            flush=True,
        )
        print(f"decide candidates={decision}", flush=True)

        ranked_now = sorted(evaluated.values(), key=lambda item: item["score"], reverse=True)
        best = ranked_now[0]
        queue = [candidate for candidate in decision if config_key(candidate) not in evaluated]
        if config_key(best["config"]) != key:
            queue.insert(0, best["config"])
        if not queue:
            queue = next_unexplored_configs(best["config"], evaluated, 6)
        if not queue:
            queue = mutate_config(best["config"], observe(best), loop_index + 1)
        print(f"act queued={queue}", flush=True)

    ranked = sorted(evaluated.values(), key=lambda item: item["score"], reverse=True)
    summary_path = output_dir / "summary.json"
    loops_path = output_dir / "loops.json"
    summary_path.write_text(json.dumps(ranked, indent=2))
    loops_path.write_text(json.dumps(history, indent=2))

    print("\nTop Configs", flush=True)
    for item in ranked[:5]:
        print(
            "{index:02d} {config} score={score:.4f} fidelity={fidelity:.4f} "
            "edge_f1={edge_f1:.4f} size={size:.1f}KB paths={paths:.1f} time={time:.1f}ms".format(
                index=item["index"],
                config=item["config"],
                score=item["score"],
                fidelity=item["average_fidelity_score"],
                edge_f1=item["average_edge_f1"],
                size=item["average_file_size"] / 1024.0,
                paths=item["average_path_count"],
                time=item["average_elapsed_ms"],
            ),
            flush=True,
        )
    print(f"\nSummary\n{summary_path}\n{loops_path}", flush=True)


if __name__ == "__main__":
    main()

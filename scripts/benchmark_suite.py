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
        "work_dir": "benchmark_runs/golden_full",
    },
}


def run(cmd, cwd: Path) -> None:
    subprocess.run(cmd, cwd=cwd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def load_json(path: Path) -> dict:
    return json.loads(path.read_text())


def line(char: str = "=", width: int = 88) -> str:
    return char * width


def format_kb(value: float) -> str:
    return f"{value / 1024.0:.1f}KB"


def verdict(delta: float, lower_is_better: bool) -> str:
    better = delta < 0 if lower_is_better else delta > 0
    if abs(delta) < 1e-9:
        return "flat"
    return "better" if better else "worse"


def quality_buckets(entries: list[dict]) -> dict:
    fidelity = [entry["report"]["metrics"]["fidelity_score"] for entry in entries]
    return {
        "critical": sum(value < 0.75 for value in fidelity),
        "watch": sum(0.75 <= value < 0.85 for value in fidelity),
        "strong": sum(value >= 0.90 for value in fidelity),
    }


def entry_id(entry: dict) -> str:
    return entry.get("reference") or Path(entry["input"]).name


def validate_baseline(current: dict, baseline: dict) -> tuple[bool, str]:
    required = ["average_fidelity_score", "average_ssim", "entries"]
    missing = [key for key in required if key not in baseline]
    if missing:
        return False, f"missing keys: {', '.join(missing)}"

    current_ids = {entry_id(entry) for entry in current["entries"]}
    baseline_ids = {entry_id(entry) for entry in baseline["entries"]}
    if not current_ids or not baseline_ids:
        return False, "empty entry set"

    overlap = len(current_ids & baseline_ids) / max(1, len(current_ids))
    if overlap < 0.8:
        return False, f"corpus mismatch overlap={overlap:.2f}"

    return True, "ok"


def print_report(report: dict) -> None:
    buckets = quality_buckets(report["entries"])

    print(f"\n{line()}")
    print("Overall")
    print(line("-"))
    print(
        "entries={entries} reliability={reliability:.4f} efficiency={efficiency:.4f} fidelity={fidelity:.4f} ssim={ssim:.4f} local_ssim_p10={local_ssim_p10:.4f} psnr={psnr:.2f} mae={mae:.2f} "
        "grad={gradient:.4f} edge_iou={edge:.4f} edge_f1={edge_f1:.4f} fg_iou={fg_iou:.4f} color={color:.4f} "
        "topo={topo:.4f} size={size} paths={paths:.1f} time={time:.1f}ms ips={ips:.2f}".format(
            entries=len(report["entries"]),
            reliability=report.get("average_reliability_score", 0.0),
            efficiency=report.get("average_efficiency_score", 0.0),
            fidelity=report["average_fidelity_score"],
            ssim=report["average_ssim"],
            local_ssim_p10=report.get("average_local_ssim_p10", 0.0),
            gradient=report.get("average_gradient_similarity", 0.0),
            psnr=report["average_psnr"],
            mae=report["average_mae"],
            size=format_kb(report["average_file_size"]),
            paths=report["average_path_count"],
            edge=report["average_edge_similarity"],
            edge_f1=report["average_edge_f1"],
            fg_iou=report["average_foreground_iou"],
            color=report["average_color_similarity"],
            topo=report["average_topology_score"],
            time=report["average_elapsed_ms"],
            ips=report["throughput_images_per_sec"],
        )
    )
    print(
        "quality_gates strong={strong} watch={watch} critical={critical}".format(
            strong=buckets["strong"],
            watch=buckets["watch"],
            critical=buckets["critical"],
        )
    )
    if "robust_benchmark_score" in report:
        print(f"robust_score={report['robust_benchmark_score']:.4f}")
    if "dataset_health_score" in report:
        print(f"dataset_health={report['dataset_health_score']:.4f}")
    if (
        "balanced_group_score" in report
        or "balanced_subgroup_score" in report
        or "balanced_kind_complexity_score" in report
    ):
        print(
            "balanced_scores groups={groups:.4f} subgroups={subgroups:.4f} kind_complexity={kind_complexity:.4f}".format(
                groups=report.get("balanced_group_score", 0.0),
                subgroups=report.get("balanced_subgroup_score", 0.0),
                kind_complexity=report.get("balanced_kind_complexity_score", 0.0),
            )
        )
    if "quality_gates" in report:
        gates = report["quality_gates"]
        print(
            "gate_failures fidelity<0.75={f075} fidelity<0.85={f085} local_ssim_p10<0.75={l075} edge<0.75={e075} "
            "edge<0.85={e085} color<0.75={c075} topo<0.50={t050} reliability<0.80={r080} "
            "efficiency<0.50={x050} size_budget={size_budget} path_budget={path_budget}".format(
                f075=gates.get("fidelity_below_0_75", 0),
                f085=gates.get("fidelity_below_0_85", 0),
                l075=gates.get("local_ssim_p10_below_0_75", 0),
                e075=gates.get("edge_f1_below_0_75", 0),
                e085=gates.get("edge_f1_below_0_85", 0),
                c075=gates.get("color_below_0_75", 0),
                t050=gates.get("topology_below_0_50", 0),
                r080=gates.get("reliability_below_0_80", 0),
                x050=gates.get("efficiency_below_0_50", 0),
                size_budget=gates.get("size_budget_failures", 0),
                path_budget=gates.get("path_budget_failures", 0),
            )
        )
    dataset = report.get("dataset", {})
    if dataset.get("groups"):
        print("dataset_groups=" + ", ".join(f"{item['label']}:{item['entries']}" for item in dataset["groups"]))
    if dataset.get("reference_complexities"):
        print(
            "dataset_source_complexity="
            + ", ".join(
                f"{item['label']}:{item['entries']}" for item in dataset["reference_complexities"]
            )
        )
    if dataset.get("kind_complexities"):
        print(
            "dataset_mix="
            + ", ".join(f"{item['label']}:{item['entries']}" for item in dataset["kind_complexities"])
        )

    print(f"\n{line()}")
    print("By Group")
    print(line("-"))
    for group in report["groups"]:
        print(
            "{group:14s} entries={entries:3d} reliability={reliability:.4f} efficiency={efficiency:.4f} fidelity={fidelity:.4f} ssim={ssim:.4f} local_ssim_p10={local_ssim_p10:.4f} "
            "grad={gradient:.4f} edge_f1={edge_f1:.4f} color={color:.4f} size={size:>8s} "
            "paths={paths:5.1f} time={time:6.1f}ms".format(
                group=group["group"],
                entries=group["entries"],
                reliability=group.get("average_reliability_score", 0.0),
                efficiency=group.get("average_efficiency_score", 0.0),
                fidelity=group["average_fidelity_score"],
                ssim=group["average_ssim"],
                local_ssim_p10=group.get("average_local_ssim_p10", 0.0),
                gradient=group.get("average_gradient_similarity", 0.0),
                edge_f1=group["average_edge_f1"],
                color=group["average_color_similarity"],
                size=format_kb(group["average_file_size"]),
                paths=group["average_path_count"],
                time=group["average_elapsed_ms"],
            )
        )

    worst = sorted(report["entries"], key=lambda entry: entry["report"]["metrics"]["fidelity_score"])[:5]
    print(f"\n{line()}")
    print("Lowest Fidelity Entries")
    print(line("-"))
    for entry in worst:
        metrics = entry["report"]["metrics"]
        print(
            "{name:32s} fidelity={fidelity:.4f} ssim={ssim:.4f} edge_f1={edge_f1:.4f} "
            "fg_iou={fg_iou:.4f} color={color:.4f} size={size:>8s} paths={paths}".format(
                name=(entry.get("reference") or Path(entry["input"]).name)[:32],
                fidelity=metrics["fidelity_score"],
                ssim=metrics["ssim"],
                edge_f1=metrics["edge_f1"],
                fg_iou=metrics["foreground_iou"],
                color=metrics["color_similarity"],
                size=format_kb(metrics["file_size"]),
                paths=metrics["path_count"],
            )
        )


def print_delta(current: dict, baseline: dict) -> None:
    print(f"\n{line()}")
    print("Delta vs Baseline")
    print(line("-"))
    metrics = [
        ("average_reliability_score", "reliability", False),
        ("average_efficiency_score", "efficiency", False),
        ("average_fidelity_score", "fidelity", False),
        ("average_ssim", "ssim", False),
        ("average_local_ssim_p10", "local_ssim_p10", False),
        ("average_gradient_similarity", "gradient", False),
        ("average_edge_f1", "edge_f1", False),
        ("average_foreground_iou", "fg_iou", False),
        ("average_color_similarity", "color", False),
        ("robust_benchmark_score", "robust_score", False),
        ("dataset_health_score", "dataset", False),
        ("balanced_group_score", "group_score", False),
        ("balanced_subgroup_score", "subgroup_score", False),
        ("balanced_kind_complexity_score", "kindcx_score", False),
        ("average_psnr", "psnr", False),
        ("average_mae", "mae", True),
        ("average_file_size", "size_bytes", True),
        ("average_path_count", "path_count", True),
        ("average_elapsed_ms", "elapsed_ms", True),
    ]
    for key, label, lower_is_better in metrics:
        if key not in current or key not in baseline:
            print(f"{label:12s} delta=n/a missing_baseline_metric")
            continue
        delta = current[key] - baseline[key]
        print(f"{label:12s} delta={delta:+.4f} {verdict(delta, lower_is_better)}")


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
    reports_dir = work_dir / "reports"
    json_path = reports_dir / "report.json"
    markdown_path = reports_dir / "report.md"
    work_dir.mkdir(parents=True, exist_ok=True)
    reports_dir.mkdir(parents=True, exist_ok=True)

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

    print(line())
    print("EdgeSVG Benchmark")
    print(line("-"))
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
        compatible, reason = validate_baseline(report, baseline)
        if compatible:
            print_delta(report, baseline)
        else:
            print(f"\n{line()}")
            print("Delta vs Baseline")
            print(line("-"))
            print(f"skipped incompatible baseline: {reason}")

    print(f"\n{line()}")
    print("Artifacts")
    print(line("-"))
    print(f"json={json_path}")
    print(f"markdown={markdown_path}")
    print(f"summary_dir={reports_dir / 'summary'}")
    print(f"groups_dir={reports_dir / 'groups'}")
    print(f"entries_dir={reports_dir / 'entries'}")
    print(f"failures_dir={reports_dir / 'failures'}")
    print(f"dataset_dir={reports_dir / 'dataset'}")


if __name__ == "__main__":
    main()

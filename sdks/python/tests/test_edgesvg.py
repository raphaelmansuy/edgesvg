from __future__ import annotations

import shutil
from pathlib import Path

import edgesvg


def test_vectorize_compare_optimize_and_render(tmp_path: Path) -> None:
    input_path = tmp_path / "fixture.png"
    fixture = Path(__file__).resolve().parents[3] / "examples" / "test_logo_benchmark.png"
    shutil.copyfile(fixture, input_path)

    result = edgesvg.vectorize(input_path, method="auto")
    assert "<svg" in result["svg"]
    assert result["report"]["metrics"]["path_count"] > 0

    analysis = edgesvg.analyze(input_path)
    assert analysis["analysis"]["width"] > 0

    info = edgesvg.inspect(input_path)
    assert "recommended_method" in info

    metrics = edgesvg.compare(input_path, result["svg"])
    assert metrics["ssim"] >= 0.0

    optimized = edgesvg.optimize_svg(result["svg"])
    assert optimized["optimized_svg"].startswith("<svg")

    rendered = edgesvg.render_png(result["svg"], 64, 32)
    assert rendered.startswith(b"\x89PNG")

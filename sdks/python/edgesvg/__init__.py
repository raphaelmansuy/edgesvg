"""edgesvg — Rust-native raster to SVG vectorization for Python."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Optional, Union

from edgesvg._edgesvg import (
    analyze_json as _analyze_json,
    benchmark_golden_json as _benchmark_golden_json,
    benchmark_json as _benchmark_json,
    compare_json as _compare_json,
    inspect_json as _inspect_json,
    optimize_json as _optimize_json,
    render_png_bytes as _render_png_bytes,
    vectorize_json as _vectorize_json,
    version as _version,
)
from ._types import JsonDict, QualityMetrics, VectorizeResponse

PathLike = Union[str, Path]


def _loads(payload: str) -> JsonDict:
    return json.loads(payload)


def vectorize(
    input_path: PathLike,
    *,
    method: str = "hifi",
    target_ssim: float = 0.998,
    max_file_size: int = 100_000,
    max_iterations: int = 4,
    quality: str = "ultra",
    logo_quality: Optional[str] = None,
    colors: Optional[int] = None,
) -> VectorizeResponse:
    return _loads(
        _vectorize_json(
            str(input_path),
            method=method,
            target_ssim=target_ssim,
            max_file_size=max_file_size,
            max_iterations=max_iterations,
            quality=quality,
            logo_quality=logo_quality,
            colors=colors,
        )
    )


def analyze(input_path: PathLike) -> JsonDict:
    return _loads(_analyze_json(str(input_path)))


def inspect(input_path: PathLike) -> JsonDict:
    return _loads(_inspect_json(str(input_path)))


def compare(input_path: PathLike, svg: str) -> QualityMetrics:
    return _loads(_compare_json(str(input_path), svg))


def optimize_svg(svg: str, *, precision: int = 2) -> JsonDict:
    return _loads(_optimize_json(svg, precision=precision))


def render_png(svg: str, width: int, height: int) -> bytes:
    return bytes(_render_png_bytes(svg, width, height))


def benchmark(
    input_dir: PathLike,
    output_dir: PathLike,
    *,
    target_ssim: float = 0.998,
    max_file_size: int = 100_000,
    max_iterations: int = 4,
    quality: str = "ultra",
) -> JsonDict:
    return _loads(
        _benchmark_json(
            str(input_dir),
            str(output_dir),
            target_ssim=target_ssim,
            max_file_size=max_file_size,
            max_iterations=max_iterations,
            quality=quality,
        )
    )


def benchmark_golden(
    golden_dir: PathLike,
    work_dir: PathLike,
    *,
    target_ssim: float = 0.998,
    max_file_size: int = 100_000,
    max_iterations: int = 4,
    quality: str = "figma",
    limit: Optional[int] = None,
) -> JsonDict:
    return _loads(
        _benchmark_golden_json(
            str(golden_dir),
            str(work_dir),
            target_ssim=target_ssim,
            max_file_size=max_file_size,
            max_iterations=max_iterations,
            quality=quality,
            limit=limit,
        )
    )


def version() -> str:
    return _version()


__all__ = [
    "analyze",
    "benchmark",
    "benchmark_golden",
    "compare",
    "inspect",
    "optimize_svg",
    "render_png",
    "vectorize",
    "version",
]
__version__ = _version()

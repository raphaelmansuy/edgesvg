from __future__ import annotations

from typing import Any, Dict, TypedDict


JsonDict = Dict[str, Any]


class QualityMetrics(TypedDict, total=False):
    ssim: float
    ssim_perceptual: float
    edge_similarity: float
    edge_precision: float
    edge_recall: float
    edge_f1: float
    foreground_iou: float
    color_similarity: float
    fidelity_score: float
    delta_e: float
    topology_score: float
    psnr: float
    mae: float
    file_size: int
    path_count: int


class VectorizeResponse(TypedDict, total=False):
    svg: str
    report: JsonDict
    requested_method: str
    effective_method: str
    fallback_from: str | None
    decision: JsonDict | None

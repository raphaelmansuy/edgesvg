/** Shared types for the EdgeSVG demo app. */

export type VectorizeMethod = 'hifi' | 'logo' | 'premium' | 'auto' | 'smart' | 'optimal' | 'bayesian' | 'sam';
export type QualityPreset = 'figma' | 'balanced' | 'quality' | 'ultra';
export type LogoQualityPreset = 'clean' | 'balanced' | 'high' | 'ultra';

export interface VectorizeRequest {
  method?: VectorizeMethod;
  target_ssim?: number;
  max_file_size?: number;
  max_iterations?: number;
  quality?: QualityPreset;
  logo_quality?: LogoQualityPreset | null;
  colors?: number | null;
}

export interface ImageAnalysis {
  width: number;
  height: number;
  unique_colors: number;
  top_10_coverage: number;
  top_50_coverage: number;
  color_variance: number;
  edge_density: number;
  dominant_colors: string[];
  image_type: 'logo' | 'icon' | 'illustration' | 'photo';
  complexity: 'simple' | 'medium' | 'complex';
}

export interface QualityMetrics {
  ssim: number;
  ssim_perceptual: number;
  edge_similarity: number;
  edge_precision: number;
  edge_recall: number;
  edge_f1: number;
  foreground_iou: number;
  color_similarity: number;
  fidelity_score: number;
  delta_e: number;
  topology_score: number;
  psnr: number;
  mae: number;
  file_size: number;
  path_count: number;
}

export interface VectorizationReport {
  analysis: ImageAnalysis;
  settings: Record<string, unknown>;
  quality_preset: string;
  metrics: QualityMetrics;
}

export interface VectorizeResponse {
  svg: string;
  report: VectorizationReport;
  requested_method: string;
  effective_method: string;
  fallback_from: string | null;
  decision: { mode: string; reason: string } | null;
}

export interface AnalyzeResponse {
  analysis: ImageAnalysis;
  decision: { mode: string; reason: string };
}

export type WasmStatus = 'idle' | 'loading' | 'ready' | 'error';
export type ProcessStatus = 'idle' | 'analyzing' | 'vectorizing' | 'done' | 'error';
export type OutputTab = 'svg' | 'code' | 'info';

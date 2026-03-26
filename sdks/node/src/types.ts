export interface VectorizeOptions {
  method?: 'hifi' | 'logo' | 'premium' | 'auto' | 'smart' | 'optimal' | 'bayesian' | 'sam';
  targetSsim?: number;
  maxFileSize?: number;
  maxIterations?: number;
  quality?: 'figma' | 'balanced' | 'quality' | 'ultra';
  logoQuality?: 'clean' | 'balanced' | 'high' | 'ultra';
  colors?: number;
}

export interface BenchmarkOptions {
  targetSsim?: number;
  maxFileSize?: number;
  maxIterations?: number;
  quality?: 'figma' | 'balanced' | 'quality' | 'ultra';
  limit?: number;
}

export interface QualityMetrics {
  ssim: number;
  fidelity_score: number;
  file_size: number;
  path_count: number;
}

export interface VectorizeResponse {
  svg: string;
  report: Record<string, unknown>;
  requested_method: string;
  effective_method: string;
  fallback_from: string | null;
  decision: Record<string, unknown> | null;
}

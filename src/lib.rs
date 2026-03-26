pub mod analysis;
pub mod benchmark;
pub mod highlevel;
pub mod metrics;
pub mod pipeline;
pub mod preprocess;
pub mod sdk;
pub mod svg;
pub mod types;
mod vectorizer;

pub use analysis::analyze_image;
pub use benchmark::{benchmark_directory, benchmark_golden_data, BenchmarkReport};
pub use highlevel::{
    determine_auto_mode, determine_auto_mode_image, is_monochrome_icon, vectorize_auto,
    vectorize_auto_image, vectorize_logo_premium, vectorize_logo_premium_image, vectorize_optimal,
    vectorize_optimal_image, vectorize_premium, vectorize_premium_image, vectorize_smart,
    vectorize_smart_image, AutoDecision,
};
pub use metrics::{compute_metrics, render_svg_to_image, QualityMetrics};
pub use pipeline::{
    vectorize, vectorize_icon, vectorize_image, vectorize_logo, write_svg, VectorizeOptions,
};
pub use preprocess::{
    adaptive_trace_settings, count_unique_colors, preprocess_image, quantize_image,
    trace_settings_for_logo_preset,
};
pub use sdk::{
    analyze_bytes, analyze_path, benchmark, benchmark_golden, compare_bytes, compare_path,
    inspect_path, optimize, render_png, vectorize_bytes, vectorize_path, AnalyzeResponse,
    BenchmarkRequest, InfoResponse, OptimizeResponse, VectorizeRequest, VectorizeResponse,
};
pub use svg::optimize_svg;
pub use types::{
    AutoMode, Complexity, ImageAnalysis, ImageKind, LogoQualityPreset, QualityPreset,
    TraceSettings, VectorizationReport, VectorizeMethod,
};

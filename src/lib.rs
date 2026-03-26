pub mod analysis;
pub mod benchmark;
pub mod highlevel;
pub mod metrics;
pub mod pipeline;
pub mod preprocess;
pub mod svg;
pub mod types;
mod vectorizer;

pub use analysis::analyze_image;
pub use benchmark::{benchmark_directory, benchmark_golden_data, BenchmarkReport};
pub use highlevel::{
    determine_auto_mode, is_monochrome_icon, vectorize_auto, vectorize_logo_premium,
    vectorize_premium, AutoDecision,
};
pub use metrics::{compute_metrics, render_svg_to_image, QualityMetrics};
pub use pipeline::{vectorize, vectorize_icon, vectorize_logo, write_svg, VectorizeOptions};
pub use preprocess::{
    adaptive_trace_settings, count_unique_colors, preprocess_image, quantize_image,
    trace_settings_for_logo_preset,
};
pub use svg::optimize_svg;
pub use types::{
    AutoMode, Complexity, ImageAnalysis, ImageKind, LogoQualityPreset, QualityPreset,
    TraceSettings, VectorizationReport,
};

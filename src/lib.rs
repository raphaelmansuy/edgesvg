pub mod analysis;
pub mod benchmark;
pub mod metrics;
pub mod pipeline;
pub mod preprocess;
pub mod svg;
pub mod types;

pub use analysis::analyze_image;
pub use benchmark::{benchmark_directory, BenchmarkReport};
pub use metrics::{compute_metrics, render_svg_to_image, QualityMetrics};
pub use pipeline::{vectorize, vectorize_icon, vectorize_logo, write_svg, VectorizeOptions};
pub use preprocess::{
    adaptive_trace_settings, count_unique_colors, preprocess_image, quantize_image,
};
pub use svg::optimize_svg;
pub use types::{
    Complexity, ImageAnalysis, ImageKind, QualityPreset, TraceSettings, VectorizationReport,
};

use clap::ValueEnum;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageKind {
    Logo,
    Icon,
    Illustration,
    Photo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    Simple,
    Medium,
    Complex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum QualityPreset {
    Compact,
    Balanced,
    Quality,
    Ultra,
}

impl QualityPreset {
    pub fn ordered_for_iterations() -> [Self; 3] {
        [Self::Compact, Self::Balanced, Self::Quality]
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageAnalysis {
    pub width: u32,
    pub height: u32,
    pub unique_colors: usize,
    pub top_10_coverage: f64,
    pub top_50_coverage: f64,
    pub color_variance: f64,
    pub edge_density: f64,
    pub dominant_colors: Vec<String>,
    pub image_type: ImageKind,
    pub complexity: Complexity,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceSettings {
    pub filter_speckle: usize,
    pub color_precision: i32,
    pub layer_difference: i32,
    pub corner_threshold: i32,
    pub length_threshold: f64,
    pub max_iterations: usize,
    pub splice_threshold: i32,
    pub path_precision: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct VectorizationReport {
    pub analysis: ImageAnalysis,
    pub settings: TraceSettings,
    pub quality_preset: QualityPreset,
    pub metrics: crate::metrics::QualityMetrics,
}

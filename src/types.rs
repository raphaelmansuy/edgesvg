use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageKind {
    Logo,
    Icon,
    Illustration,
    Photo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    Simple,
    Medium,
    Complex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, Default)]
#[serde(rename_all = "snake_case")]
pub enum QualityPreset {
    Figma,
    #[default]
    Balanced,
    Quality,
    Ultra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogoQualityPreset {
    Clean,
    #[default]
    Balanced,
    High,
    Ultra,
}

impl QualityPreset {
    pub fn optimizer_precision(self) -> u32 {
        match self {
            Self::Figma => 1,
            Self::Balanced | Self::Quality | Self::Ultra => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoMode {
    Logo,
    Premium,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceMode {
    Spline,
    Polygon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, Default)]
#[serde(rename_all = "snake_case")]
pub enum VectorizeMethod {
    #[default]
    Hifi,
    Logo,
    Premium,
    Auto,
    Smart,
    Optimal,
    Bayesian,
    Sam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAnalysis {
    pub width: u32,
    pub height: u32,
    pub unique_colors: usize,
    pub effective_colors: usize,
    pub top_10_coverage: f64,
    pub top_50_coverage: f64,
    pub color_variance: f64,
    pub edge_density: f64,
    pub alpha_coverage: f64,
    pub dominant_colors: Vec<String>,
    pub image_type: ImageKind,
    pub complexity: Complexity,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceSettings {
    pub color_mode: &'static str,
    pub hierarchical: &'static str,
    pub mode: TraceMode,
    pub filter_speckle: usize,
    pub color_precision: i32,
    pub layer_difference: i32,
    pub length_threshold: f64,
    pub corner_threshold: i32,
    pub max_iterations: usize,
    pub splice_threshold: i32,
    pub path_precision: u32,
    pub optimizer_precision: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct VectorizationReport {
    pub analysis: ImageAnalysis,
    pub settings: TraceSettings,
    pub quality_preset: QualityPreset,
    pub metrics: crate::metrics::QualityMetrics,
}

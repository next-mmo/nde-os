//! Figma JSON Render Schema — Rust Types
//!
//! Serde-serializable structs that mirror the FDocument JSON schema.
//! Used for parsing, conversion, and IPC transport to the frontend.

#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};

// ─── Primitive value types ───────────────────────────────────────────

/// RGBA color with 0.0–1.0 float channels.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl FColor {
    pub fn to_css(&self) -> String {
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        format!("rgba({r}, {g}, {b}, {})", self.a)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FGradientStop {
    pub position: f64,
    pub color: FColor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FFill {
    SOLID {
        color: FColor,
        #[serde(default = "default_opacity")]
        opacity: f64,
    },
    LINEAR_GRADIENT {
        stops: Vec<FGradientStop>,
        #[serde(default = "default_angle")]
        angle: f64,
    },
    RADIAL_GRADIENT {
        stops: Vec<FGradientStop>,
    },
    IMAGE {
        src: String,
        #[serde(default = "default_scale_mode")]
        #[serde(rename = "scaleMode")]
        scale_mode: String,
    },
}

fn default_opacity() -> f64 { 1.0 }
fn default_angle() -> f64 { 180.0 }
fn default_scale_mode() -> String { "FILL".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FStroke {
    pub color: FColor,
    pub weight: f64,
    #[serde(default)]
    pub align: Option<String>,
    #[serde(default, rename = "dashPattern")]
    pub dash_pattern: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FEffect {
    DROP_SHADOW {
        color: FColor,
        offset: FOffset,
        radius: f64,
        #[serde(default)]
        spread: Option<f64>,
    },
    INNER_SHADOW {
        color: FColor,
        offset: FOffset,
        radius: f64,
        #[serde(default)]
        spread: Option<f64>,
    },
    LAYER {
        radius: f64,
    },
    BACKGROUND {
        radius: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FOffset {
    pub x: f64,
    pub y: f64,
}

// ─── Node types ──────────────────────────────────────────────────────

/// Base properties shared across all node types.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FBaseProps {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    #[serde(default)]
    pub rotation: Option<f64>,
    #[serde(default, rename = "borderRadius")]
    pub border_radius: Option<FBorderRadius>,
    #[serde(default, rename = "clipsContent")]
    pub clips_content: bool,
    #[serde(default, rename = "blendMode")]
    pub blend_mode: Option<String>,
    #[serde(default)]
    pub fills: Vec<FFill>,
    #[serde(default)]
    pub strokes: Vec<FStroke>,
    #[serde(default)]
    pub effects: Vec<FEffect>,
    #[serde(default, rename = "onClick")]
    pub on_click: Option<String>,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FBorderRadius {
    Uniform(f64),
    PerCorner([f64; 4]),
}

/// An FNode is a tagged enum of all renderable node types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FNode {
    FRAME(FFrameData),
    GROUP(FFrameData),
    COMPONENT(FFrameData),
    INSTANCE(FFrameData),
    TEXT(FTextData),
    RECTANGLE(FShapeData),
    ELLIPSE(FShapeData),
    VECTOR(FVectorData),
    LINE(FVectorData),
    STAR(FVectorData),
    POLYGON(FVectorData),
    IMAGE(FImageData),
}

impl FNode {
    pub fn base(&self) -> &FBaseProps {
        match self {
            FNode::FRAME(d) | FNode::GROUP(d)
            | FNode::COMPONENT(d) | FNode::INSTANCE(d) => &d.base,
            FNode::TEXT(d) => &d.base,
            FNode::RECTANGLE(d) | FNode::ELLIPSE(d) => &d.base,
            FNode::VECTOR(d) | FNode::LINE(d)
            | FNode::STAR(d) | FNode::POLYGON(d) => &d.base,
            FNode::IMAGE(d) => &d.base,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            FNode::FRAME(_) => "FRAME",
            FNode::GROUP(_) => "GROUP",
            FNode::COMPONENT(_) => "COMPONENT",
            FNode::INSTANCE(_) => "INSTANCE",
            FNode::TEXT(_) => "TEXT",
            FNode::RECTANGLE(_) => "RECTANGLE",
            FNode::ELLIPSE(_) => "ELLIPSE",
            FNode::VECTOR(_) => "VECTOR",
            FNode::LINE(_) => "LINE",
            FNode::STAR(_) => "STAR",
            FNode::POLYGON(_) => "POLYGON",
            FNode::IMAGE(_) => "IMAGE",
        }
    }
}

// ─── Frame data ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FFrameData {
    #[serde(flatten)]
    pub base: FBaseProps,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default, rename = "layoutMode")]
    pub layout_mode: Option<String>,
    #[serde(default, rename = "layoutWrap")]
    pub layout_wrap: Option<String>,
    #[serde(default, rename = "primaryAxisAlignItems")]
    pub primary_axis_align: Option<String>,
    #[serde(default, rename = "counterAxisAlignItems")]
    pub counter_axis_align: Option<String>,
    #[serde(default, rename = "primaryAxisSizingMode")]
    pub primary_axis_sizing: Option<String>,
    #[serde(default, rename = "counterAxisSizingMode")]
    pub counter_axis_sizing: Option<String>,
    #[serde(default, rename = "layoutGrow")]
    pub layout_grow: Option<f64>,
    #[serde(default, rename = "itemSpacing")]
    pub item_spacing: Option<f64>,
    #[serde(default, rename = "counterAxisSpacing")]
    pub counter_axis_spacing: Option<f64>,
    #[serde(default, rename = "paddingLeft")]
    pub padding_left: Option<f64>,
    #[serde(default, rename = "paddingRight")]
    pub padding_right: Option<f64>,
    #[serde(default, rename = "paddingTop")]
    pub padding_top: Option<f64>,
    #[serde(default, rename = "paddingBottom")]
    pub padding_bottom: Option<f64>,
    #[serde(default)]
    pub children: Vec<FNode>,
}

// ─── Text data ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FTextData {
    #[serde(flatten)]
    pub base: FBaseProps,
    pub characters: String,
    #[serde(default, rename = "fontFamily")]
    pub font_family: Option<String>,
    #[serde(default, rename = "fontWeight")]
    pub font_weight: Option<f64>,
    #[serde(default, rename = "fontSize")]
    pub font_size: Option<f64>,
    #[serde(default, rename = "lineHeight")]
    pub line_height: Option<FLineHeight>,
    #[serde(default, rename = "letterSpacing")]
    pub letter_spacing: Option<f64>,
    #[serde(default, rename = "textDecoration")]
    pub text_decoration: Option<String>,
    #[serde(default, rename = "textCase")]
    pub text_case: Option<String>,
    #[serde(default, rename = "textAlignHorizontal")]
    pub text_align_horizontal: Option<String>,
    #[serde(default, rename = "textAlignVertical")]
    pub text_align_vertical: Option<String>,
    #[serde(default)]
    pub width: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FLineHeight {
    Px(f64),
    Auto(String), // "AUTO"
}

// ─── Shape data ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FShapeData {
    #[serde(flatten)]
    pub base: FBaseProps,
    pub width: f64,
    pub height: f64,
}

// ─── Vector data ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FVectorData {
    #[serde(flatten)]
    pub base: FBaseProps,
    pub width: f64,
    pub height: f64,
    #[serde(default, rename = "svgPath")]
    pub svg_path: Option<String>,
}

// ─── Image data ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FImageData {
    #[serde(flatten)]
    pub base: FBaseProps,
    pub width: f64,
    pub height: f64,
    pub src: String,
    #[serde(default)]
    pub alt: Option<String>,
    #[serde(default, rename = "objectFit")]
    pub object_fit: Option<String>,
}

// ─── Document root ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FDocumentMeta {
    #[serde(default, rename = "figmaFileKey")]
    pub figma_file_key: Option<String>,
    #[serde(default, rename = "figmaNodeId")]
    pub figma_node_id: Option<String>,
    #[serde(default, rename = "exportedAt")]
    pub exported_at: Option<String>,
    #[serde(default)]
    pub generator: Option<String>,
}

/// The root document that the renderer consumes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FDocument {
    pub version: String,
    pub name: String,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub background: Option<FColor>,
    pub children: Vec<FNode>,
    #[serde(default)]
    pub meta: Option<FDocumentMeta>,
}

// ─── Sample document ─────────────────────────────────────────────────

impl FDocument {
    /// Returns a sample dark-themed card for testing.
    pub fn sample() -> Self {
        // Parse the sample from embedded JSON — ensures it stays in sync
        // with what the renderer expects.
        serde_json::from_str(include_str!("sample.json"))
            .expect("embedded sample.json must be valid")
    }
}

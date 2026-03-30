//! FNode → CSS inline style resolver (Rust)
//!
//! Converts FNode properties into a `BTreeMap<String, String>` of CSS
//! key-value pairs. The frontend receives this pre-computed map via IPC
//! and applies it directly as inline styles — zero JS computation needed.

use std::collections::BTreeMap;

use super::types::*;

type StyleMap = BTreeMap<String, String>;

// ─── Fill → CSS background ──────────────────────────────────────────

fn fill_to_css(fill: &FFill) -> Option<String> {
    match fill {
        FFill::SOLID { color, opacity } => {
            let c = FColor {
                r: color.r,
                g: color.g,
                b: color.b,
                a: *opacity,
            };
            Some(c.to_css())
        }
        FFill::LINEAR_GRADIENT { stops, angle } => {
            let stop_str: Vec<String> = stops
                .iter()
                .map(|s| format!("{} {:.1}%", s.color.to_css(), s.position * 100.0))
                .collect();
            Some(format!(
                "linear-gradient({angle}deg, {})",
                stop_str.join(", ")
            ))
        }
        FFill::RADIAL_GRADIENT { stops } => {
            let stop_str: Vec<String> = stops
                .iter()
                .map(|s| format!("{} {:.1}%", s.color.to_css(), s.position * 100.0))
                .collect();
            Some(format!("radial-gradient(circle, {})", stop_str.join(", ")))
        }
        FFill::IMAGE { src, .. } => Some(format!("url({src})")),
    }
}

fn resolve_fills(fills: &[FFill]) -> StyleMap {
    let mut m = StyleMap::new();
    if fills.is_empty() {
        return m;
    }

    // Single solid fill → background-color (most common, fastest)
    if fills.len() == 1 {
        if let FFill::SOLID { .. } = &fills[0] {
            if let Some(css) = fill_to_css(&fills[0]) {
                m.insert("background-color".into(), css);
                return m;
            }
        }
    }

    // Multiple fills or gradients → layered background
    let bgs: Vec<String> = fills.iter().rev().filter_map(fill_to_css).collect();
    if !bgs.is_empty() {
        // Check for IMAGE fill
        let has_image = fills.iter().any(|f| matches!(f, FFill::IMAGE { .. }));
        if has_image {
            m.insert("background".into(), bgs.join(", "));
            let img = fills.iter().find(|f| matches!(f, FFill::IMAGE { .. }));
            if let Some(FFill::IMAGE { scale_mode, .. }) = img {
                let mode = match scale_mode.as_str() {
                    "FIT" => "contain",
                    "CROP" | "FILL" => "cover",
                    _ => "cover",
                };
                m.insert("background-size".into(), mode.into());
                m.insert("background-position".into(), "center".into());
                m.insert(
                    "background-repeat".into(),
                    if scale_mode == "TILE" {
                        "repeat"
                    } else {
                        "no-repeat"
                    }
                    .into(),
                );
            }
        } else {
            m.insert("background".into(), bgs.join(", "));
        }
    }
    m
}

// ─── Strokes → CSS borders ──────────────────────────────────────────

fn resolve_strokes(strokes: &[FStroke]) -> StyleMap {
    let mut m = StyleMap::new();
    if strokes.is_empty() {
        return m;
    }
    let s = &strokes[0];
    let color = s.color.to_css();
    let align = s.align.as_deref().unwrap_or("CENTER");

    match align {
        "INSIDE" => {
            m.insert(
                "box-shadow".into(),
                format!("inset 0 0 0 {}px {color}", s.weight),
            );
        }
        "OUTSIDE" => {
            m.insert("box-shadow".into(), format!("0 0 0 {}px {color}", s.weight));
        }
        _ => {
            let style = if s.dash_pattern.as_ref().map_or(false, |d| !d.is_empty()) {
                "dashed"
            } else {
                "solid"
            };
            m.insert("border".into(), format!("{}px {style} {color}", s.weight));
        }
    }
    m
}

// ─── Effects → CSS shadows + filters ────────────────────────────────

fn resolve_effects(effects: &[FEffect]) -> StyleMap {
    let mut m = StyleMap::new();
    if effects.is_empty() {
        return m;
    }

    let mut shadows = Vec::new();
    let mut filters = Vec::new();
    let mut backdrop_filters = Vec::new();

    for e in effects {
        match e {
            FEffect::DROP_SHADOW {
                color,
                offset,
                radius,
                spread,
            } => {
                let sp = spread.unwrap_or(0.0);
                shadows.push(format!(
                    "{}px {}px {}px {}px {}",
                    offset.x,
                    offset.y,
                    radius,
                    sp,
                    color.to_css()
                ));
            }
            FEffect::INNER_SHADOW {
                color,
                offset,
                radius,
                spread,
            } => {
                let sp = spread.unwrap_or(0.0);
                shadows.push(format!(
                    "inset {}px {}px {}px {}px {}",
                    offset.x,
                    offset.y,
                    radius,
                    sp,
                    color.to_css()
                ));
            }
            FEffect::LAYER { radius } => {
                filters.push(format!("blur({radius}px)"));
            }
            FEffect::BACKGROUND { radius } => {
                backdrop_filters.push(format!("blur({radius}px)"));
            }
        }
    }

    if !shadows.is_empty() {
        m.insert("box-shadow".into(), shadows.join(", "));
    }
    if !filters.is_empty() {
        m.insert("filter".into(), filters.join(" "));
    }
    if !backdrop_filters.is_empty() {
        m.insert("backdrop-filter".into(), backdrop_filters.join(" "));
    }
    m
}

// ─── Border radius ──────────────────────────────────────────────────

fn resolve_border_radius(br: &Option<FBorderRadius>) -> StyleMap {
    let mut m = StyleMap::new();
    match br {
        Some(FBorderRadius::Uniform(v)) if *v > 0.0 => {
            m.insert("border-radius".into(), format!("{v}px"));
        }
        Some(FBorderRadius::PerCorner(corners)) => {
            m.insert(
                "border-radius".into(),
                format!(
                    "{}px {}px {}px {}px",
                    corners[0], corners[1], corners[2], corners[3]
                ),
            );
        }
        _ => {}
    }
    m
}

// ─── Auto-layout → CSS flexbox ──────────────────────────────────────

fn map_align(align: Option<&str>) -> &'static str {
    match align {
        Some("MIN") => "flex-start",
        Some("MAX") => "flex-end",
        Some("CENTER") => "center",
        Some("STRETCH") => "stretch",
        Some("BASELINE") => "baseline",
        _ => "flex-start",
    }
}

fn resolve_layout(frame: &FFrameData) -> StyleMap {
    let mut m = StyleMap::new();
    let mode = frame.layout_mode.as_deref().unwrap_or("NONE");

    if mode == "HORIZONTAL" || mode == "VERTICAL" {
        m.insert("display".into(), "flex".into());
        m.insert(
            "flex-direction".into(),
            if mode == "HORIZONTAL" {
                "row"
            } else {
                "column"
            }
            .into(),
        );

        if frame.layout_wrap.as_deref() == Some("WRAP") {
            m.insert("flex-wrap".into(), "wrap".into());
        }

        m.insert(
            "justify-content".into(),
            map_align(frame.primary_axis_align.as_deref()).into(),
        );
        m.insert(
            "align-items".into(),
            map_align(frame.counter_axis_align.as_deref()).into(),
        );

        if let Some(gap) = frame.item_spacing {
            if gap > 0.0 {
                m.insert("gap".into(), format!("{gap}px"));
            }
        }
        if let Some(row_gap) = frame.counter_axis_spacing {
            if row_gap > 0.0 {
                m.insert("row-gap".into(), format!("{row_gap}px"));
            }
        }
    }

    // Padding
    let pl = frame.padding_left.unwrap_or(0.0);
    let pr = frame.padding_right.unwrap_or(0.0);
    let pt = frame.padding_top.unwrap_or(0.0);
    let pb = frame.padding_bottom.unwrap_or(0.0);
    if pl > 0.0 || pr > 0.0 || pt > 0.0 || pb > 0.0 {
        m.insert("padding".into(), format!("{pt}px {pr}px {pb}px {pl}px"));
    }

    // Sizing
    let is_hug = frame.primary_axis_sizing.as_deref() == Some("HUG")
        || frame.counter_axis_sizing.as_deref() == Some("HUG");
    if !is_hug {
        if let Some(w) = frame.width {
            m.insert("width".into(), format!("{w}px"));
        }
        if let Some(h) = frame.height {
            m.insert("height".into(), format!("{h}px"));
        }
    }

    if let Some(grow) = frame.layout_grow {
        if grow > 0.0 {
            m.insert("flex-grow".into(), format!("{grow}"));
        }
    }

    m
}

// ─── Text styles ────────────────────────────────────────────────────

fn resolve_text_styles(text: &FTextData) -> StyleMap {
    let mut m = StyleMap::new();

    if let Some(ref ff) = text.font_family {
        m.insert("font-family".into(), format!("'{ff}', sans-serif"));
    }
    if let Some(fw) = text.font_weight {
        m.insert("font-weight".into(), format!("{fw}"));
    }
    if let Some(fs) = text.font_size {
        m.insert("font-size".into(), format!("{fs}px"));
    }
    match &text.line_height {
        Some(FLineHeight::Px(px)) => {
            m.insert("line-height".into(), format!("{px}px"));
        }
        Some(FLineHeight::Auto(_)) => {
            m.insert("line-height".into(), "normal".into());
        }
        None => {}
    }
    if let Some(ls) = text.letter_spacing {
        if ls != 0.0 {
            m.insert("letter-spacing".into(), format!("{ls}px"));
        }
    }
    if let Some(ref td) = text.text_decoration {
        match td.as_str() {
            "UNDERLINE" => {
                m.insert("text-decoration".into(), "underline".into());
            }
            "STRIKETHROUGH" => {
                m.insert("text-decoration".into(), "line-through".into());
            }
            _ => {}
        }
    }
    if let Some(ref tc) = text.text_case {
        match tc.as_str() {
            "UPPER" => {
                m.insert("text-transform".into(), "uppercase".into());
            }
            "LOWER" => {
                m.insert("text-transform".into(), "lowercase".into());
            }
            "TITLE" => {
                m.insert("text-transform".into(), "capitalize".into());
            }
            _ => {}
        }
    }
    if let Some(ref ta) = text.text_align_horizontal {
        let align = match ta.as_str() {
            "CENTER" => "center",
            "RIGHT" => "right",
            "JUSTIFIED" => "justify",
            _ => "left",
        };
        m.insert("text-align".into(), align.into());
    }
    if let Some(w) = text.width {
        m.insert("max-width".into(), format!("{w}px"));
    }

    m
}

// ─── Master resolver ────────────────────────────────────────────────

/// Resolve all CSS styles for a single FNode.
/// Returns a sorted map of CSS property → value.
pub fn resolve_node_styles(node: &FNode) -> StyleMap {
    let base = node.base();
    let mut m = StyleMap::new();

    // Visibility
    if !base.visible {
        m.insert("display".into(), "none".into());
        return m;
    }

    // Opacity
    if base.opacity < 1.0 {
        m.insert("opacity".into(), format!("{}", base.opacity));
    }

    // Rotation
    if let Some(rot) = base.rotation {
        if rot != 0.0 {
            m.insert("transform".into(), format!("rotate({rot}deg)"));
        }
    }

    // Blend mode
    if let Some(ref bm) = base.blend_mode {
        if bm != "NORMAL" && bm != "PASS_THROUGH" {
            m.insert("mix-blend-mode".into(), bm.to_lowercase().replace('_', "-"));
        }
    }

    // Common visual
    m.extend(resolve_fills(&base.fills));
    m.extend(resolve_strokes(&base.strokes));
    m.extend(resolve_effects(&base.effects));
    m.extend(resolve_border_radius(&base.border_radius));

    // Clip
    if base.clips_content {
        m.insert("overflow".into(), "hidden".into());
    }

    // Type-specific
    match node {
        FNode::FRAME(d) | FNode::GROUP(d) | FNode::COMPONENT(d) | FNode::INSTANCE(d) => {
            m.extend(resolve_layout(d));
            let mode = d.layout_mode.as_deref().unwrap_or("NONE");
            if mode == "NONE" {
                m.insert("position".into(), "relative".into());
                if let Some(w) = d.width {
                    m.insert("width".into(), format!("{w}px"));
                }
                if let Some(h) = d.height {
                    m.insert("height".into(), format!("{h}px"));
                }
            }
        }
        FNode::TEXT(d) => {
            m.extend(resolve_text_styles(d));
        }
        FNode::RECTANGLE(d) => {
            m.insert("width".into(), format!("{}px", d.width));
            m.insert("height".into(), format!("{}px", d.height));
        }
        FNode::ELLIPSE(d) => {
            m.insert("width".into(), format!("{}px", d.width));
            m.insert("height".into(), format!("{}px", d.height));
            m.insert("border-radius".into(), "50%".into());
        }
        FNode::VECTOR(d) | FNode::LINE(d) | FNode::STAR(d) | FNode::POLYGON(d) => {
            m.insert("width".into(), format!("{}px", d.width));
            m.insert("height".into(), format!("{}px", d.height));
        }
        FNode::IMAGE(d) => {
            m.insert("width".into(), format!("{}px", d.width));
            m.insert("height".into(), format!("{}px", d.height));
        }
    }

    m
}

/// Resolve styles for a node and all its children recursively.
/// Returns a map of node-id → CSS style map.
pub fn resolve_all_styles(node: &FNode) -> BTreeMap<String, StyleMap> {
    let mut result = BTreeMap::new();
    resolve_recursive(node, &mut result);
    result
}

fn resolve_recursive(node: &FNode, out: &mut BTreeMap<String, StyleMap>) {
    out.insert(node.base().id.clone(), resolve_node_styles(node));

    if let FNode::FRAME(d) | FNode::GROUP(d) | FNode::COMPONENT(d) | FNode::INSTANCE(d) = node {
        for child in &d.children {
            resolve_recursive(child, out);
        }
    }
}

/// Resolve styles for an entire document.
/// Returns a map of node-id → CSS string.
pub fn resolve_document_styles(doc: &FDocument) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for child in &doc.children {
        let styles = resolve_all_styles(child);
        for (id, map) in styles {
            out.insert(id, styles_to_string(&map));
        }
    }
    out
}

/// Convert a style map to a CSS inline string.
pub fn styles_to_string(styles: &StyleMap) -> String {
    styles
        .iter()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_fill_becomes_background_color() {
        let fills = vec![FFill::SOLID {
            color: FColor {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            opacity: 1.0,
        }];
        let m = resolve_fills(&fills);
        assert_eq!(
            m.get("background-color"),
            Some(&"rgba(255, 0, 0, 1)".to_string())
        );
    }

    #[test]
    fn linear_gradient_becomes_background() {
        let fills = vec![FFill::LINEAR_GRADIENT {
            stops: vec![
                FGradientStop {
                    position: 0.0,
                    color: FColor {
                        r: 1.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    },
                },
                FGradientStop {
                    position: 1.0,
                    color: FColor {
                        r: 0.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    },
                },
            ],
            angle: 90.0,
        }];
        let m = resolve_fills(&fills);
        let bg = m.get("background").expect("should have background");
        assert!(bg.contains("linear-gradient"));
    }

    #[test]
    fn inside_stroke_becomes_inset_shadow() {
        let strokes = vec![FStroke {
            color: FColor {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.1,
            },
            weight: 1.0,
            align: Some("INSIDE".into()),
            dash_pattern: None,
        }];
        let m = resolve_strokes(&strokes);
        let shadow = m.get("box-shadow").expect("should have box-shadow");
        assert!(shadow.contains("inset"));
    }

    #[test]
    fn sample_document_parses() {
        let doc = FDocument::sample();
        assert_eq!(doc.version, "1.0");
        assert!(!doc.children.is_empty());
    }

    #[test]
    fn resolve_document_styles_produces_css() {
        let doc = FDocument::sample();
        let styles = resolve_document_styles(&doc);
        assert!(!styles.is_empty());
        // Card frame should have display: flex
        let card_style = styles.get("card-frame").expect("card-frame styles");
        assert!(card_style.contains("display: flex"));
    }
}

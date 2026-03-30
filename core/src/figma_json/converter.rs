//! Figma REST API JSON → FDocument converter (Rust)
//!
//! Converts raw Figma file/node API responses into our FDocument schema.
//! All heavy parsing runs in Rust; the frontend receives ready-to-render JSON.

use std::collections::HashMap;

use serde_json::Value;

use super::types::*;

// ─── Helpers ─────────────────────────────────────────────────────────

fn val_f64(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

fn val_str<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|v| v.as_str())
}

fn val_bool(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

fn parse_color(v: &Value) -> FColor {
    FColor {
        r: val_f64(v, "r"),
        g: val_f64(v, "g"),
        b: val_f64(v, "b"),
        a: v.get("a").and_then(|a| a.as_f64()).unwrap_or(1.0),
    }
}

fn parse_gradient_stops(v: &Value) -> Vec<FGradientStop> {
    v.get("gradientStops")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .map(|s| FGradientStop {
                    position: val_f64(s, "position"),
                    color: s.get("color").map(parse_color).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn gradient_angle(v: &Value) -> f64 {
    let handles = v.get("gradientHandlePositions").and_then(|h| h.as_array());
    match handles {
        Some(arr) if arr.len() >= 2 => {
            let dx = val_f64(&arr[1], "x") - val_f64(&arr[0], "x");
            let dy = val_f64(&arr[1], "y") - val_f64(&arr[0], "y");
            (dy.atan2(dx).to_degrees() + 90.0).round()
        }
        _ => 180.0,
    }
}

fn bounding_box_dim(node: &Value, key: &str) -> Option<f64> {
    node.get("absoluteBoundingBox")
        .and_then(|bb| bb.get(key))
        .and_then(|v| v.as_f64())
        .or_else(|| {
            node.get("size")
                .and_then(|s| s.get(if key == "width" { "x" } else { "y" }))
                .and_then(|v| v.as_f64())
        })
}

// ─── Fill converter ──────────────────────────────────────────────────

fn convert_fills(fills: &Value, image_map: &HashMap<String, String>) -> Vec<FFill> {
    let arr = match fills.as_array() {
        Some(a) => a,
        None => return vec![],
    };
    arr.iter()
        .filter(|f| val_bool(f, "visible", true))
        .filter_map(|f| {
            let typ = val_str(f, "type")?;
            match typ {
                "SOLID" => Some(FFill::SOLID {
                    color: f.get("color").map(parse_color).unwrap_or_default(),
                    opacity: f.get("opacity").and_then(|o| o.as_f64()).unwrap_or(1.0),
                }),
                "GRADIENT_LINEAR" => Some(FFill::LINEAR_GRADIENT {
                    stops: parse_gradient_stops(f),
                    angle: gradient_angle(f),
                }),
                "GRADIENT_RADIAL" | "GRADIENT_ANGULAR" | "GRADIENT_DIAMOND" => {
                    Some(FFill::RADIAL_GRADIENT {
                        stops: parse_gradient_stops(f),
                    })
                }
                "IMAGE" => {
                    let image_ref = val_str(f, "imageRef").unwrap_or("");
                    let src = image_map
                        .get(image_ref)
                        .cloned()
                        .unwrap_or_else(|| format!("figma://image/{image_ref}"));
                    Some(FFill::IMAGE {
                        src,
                        scale_mode: val_str(f, "scaleMode").unwrap_or("FILL").to_string(),
                    })
                }
                _ => None,
            }
        })
        .collect()
}

// ─── Stroke converter ───────────────────────────────────────────────

fn convert_strokes(node: &Value) -> Vec<FStroke> {
    let arr = match node.get("strokes").and_then(|s| s.as_array()) {
        Some(a) => a,
        None => return vec![],
    };
    let weight = node
        .get("strokeWeight")
        .and_then(|w| w.as_f64())
        .unwrap_or(1.0);
    let align = node.get("strokeAlign").and_then(|a| a.as_str());

    arr.iter()
        .filter(|s| val_bool(s, "visible", true))
        .map(|s| FStroke {
            color: s.get("color").map(parse_color).unwrap_or_default(),
            weight,
            align: align.map(|a| a.to_string()),
            dash_pattern: s.get("dashPattern").and_then(|d| {
                d.as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
            }),
        })
        .collect()
}

// ─── Effect converter ───────────────────────────────────────────────

fn convert_effects(effects: &Value) -> Vec<FEffect> {
    let arr = match effects.as_array() {
        Some(a) => a,
        None => return vec![],
    };
    arr.iter()
        .filter(|e| val_bool(e, "visible", true))
        .filter_map(|e| {
            let typ = val_str(e, "type")?;
            match typ {
                "DROP_SHADOW" => Some(FEffect::DROP_SHADOW {
                    color: e.get("color").map(parse_color).unwrap_or_default(),
                    offset: FOffset {
                        x: e.get("offset").map(|o| val_f64(o, "x")).unwrap_or(0.0),
                        y: e.get("offset").map(|o| val_f64(o, "y")).unwrap_or(0.0),
                    },
                    radius: val_f64(e, "radius"),
                    spread: e.get("spread").and_then(|s| s.as_f64()),
                }),
                "INNER_SHADOW" => Some(FEffect::INNER_SHADOW {
                    color: e.get("color").map(parse_color).unwrap_or_default(),
                    offset: FOffset {
                        x: e.get("offset").map(|o| val_f64(o, "x")).unwrap_or(0.0),
                        y: e.get("offset").map(|o| val_f64(o, "y")).unwrap_or(0.0),
                    },
                    radius: val_f64(e, "radius"),
                    spread: e.get("spread").and_then(|s| s.as_f64()),
                }),
                "LAYER_BLUR" => Some(FEffect::LAYER {
                    radius: val_f64(e, "radius"),
                }),
                "BACKGROUND_BLUR" => Some(FEffect::BACKGROUND {
                    radius: val_f64(e, "radius"),
                }),
                _ => None,
            }
        })
        .collect()
}

// ─── Border radius ──────────────────────────────────────────────────

fn convert_border_radius(node: &Value) -> Option<FBorderRadius> {
    if let Some(corners) = node.get("rectangleCornerRadii").and_then(|r| r.as_array()) {
        if corners.len() == 4 {
            let r: Vec<f64> = corners.iter().filter_map(|v| v.as_f64()).collect();
            if r.len() == 4 {
                if r.iter().all(|v| *v == r[0]) {
                    return if r[0] > 0.0 {
                        Some(FBorderRadius::Uniform(r[0]))
                    } else {
                        None
                    };
                }
                return Some(FBorderRadius::PerCorner([r[0], r[1], r[2], r[3]]));
            }
        }
    }
    node.get("cornerRadius")
        .and_then(|r| r.as_f64())
        .and_then(|r| {
            if r > 0.0 {
                Some(FBorderRadius::Uniform(r))
            } else {
                None
            }
        })
}

// ─── Base props ──────────────────────────────────────────────────────

fn convert_base(node: &Value, image_map: &HashMap<String, String>) -> FBaseProps {
    let fills = node.get("fills").cloned().unwrap_or(Value::Array(vec![]));
    let effects = node.get("effects").cloned().unwrap_or(Value::Array(vec![]));

    FBaseProps {
        id: val_str(node, "id").unwrap_or("").to_string(),
        name: val_str(node, "name").unwrap_or("Untitled").to_string(),
        visible: val_bool(node, "visible", true),
        opacity: node.get("opacity").and_then(|o| o.as_f64()).unwrap_or(1.0),
        rotation: node.get("rotation").and_then(|r| r.as_f64()),
        border_radius: convert_border_radius(node),
        clips_content: val_bool(node, "clipsContent", false),
        blend_mode: val_str(node, "blendMode").map(|s| s.to_string()),
        fills: convert_fills(&fills, image_map),
        strokes: convert_strokes(node),
        effects: convert_effects(&effects),
        on_click: None,
    }
}

// ─── Node converter ─────────────────────────────────────────────────

fn convert_node(node: &Value, image_map: &HashMap<String, String>) -> Option<FNode> {
    let typ = val_str(node, "type")?;
    let base = convert_base(node, image_map);

    let frame_types = [
        "FRAME",
        "GROUP",
        "COMPONENT",
        "COMPONENT_SET",
        "INSTANCE",
        "SECTION",
    ];
    if frame_types.contains(&typ) {
        let children = convert_children(node, image_map);
        let data = FFrameData {
            base,
            width: bounding_box_dim(node, "width"),
            height: bounding_box_dim(node, "height"),
            layout_mode: val_str(node, "layoutMode").map(|s| s.to_string()),
            layout_wrap: val_str(node, "layoutWrap").map(|s| s.to_string()),
            primary_axis_align: val_str(node, "primaryAxisAlignItems").map(|s| s.to_string()),
            counter_axis_align: val_str(node, "counterAxisAlignItems").map(|s| s.to_string()),
            primary_axis_sizing: val_str(node, "primaryAxisSizingMode").map(|s| s.to_string()),
            counter_axis_sizing: val_str(node, "counterAxisSizingMode").map(|s| s.to_string()),
            layout_grow: node.get("layoutGrow").and_then(|v| v.as_f64()),
            item_spacing: node.get("itemSpacing").and_then(|v| v.as_f64()),
            counter_axis_spacing: node.get("counterAxisSpacing").and_then(|v| v.as_f64()),
            padding_left: node
                .get("paddingLeft")
                .and_then(|v| v.as_f64())
                .or_else(|| node.get("horizontalPadding").and_then(|v| v.as_f64())),
            padding_right: node
                .get("paddingRight")
                .and_then(|v| v.as_f64())
                .or_else(|| node.get("horizontalPadding").and_then(|v| v.as_f64())),
            padding_top: node
                .get("paddingTop")
                .and_then(|v| v.as_f64())
                .or_else(|| node.get("verticalPadding").and_then(|v| v.as_f64())),
            padding_bottom: node
                .get("paddingBottom")
                .and_then(|v| v.as_f64())
                .or_else(|| node.get("verticalPadding").and_then(|v| v.as_f64())),
            children,
        };
        return match typ {
            "GROUP" => Some(FNode::GROUP(data)),
            "COMPONENT" => Some(FNode::COMPONENT(data)),
            "INSTANCE" => Some(FNode::INSTANCE(data)),
            _ => Some(FNode::FRAME(data)),
        };
    }

    if typ == "TEXT" {
        let style = node.get("style").unwrap_or(&Value::Null);
        return Some(FNode::TEXT(FTextData {
            base,
            characters: val_str(node, "characters").unwrap_or("").to_string(),
            font_family: val_str(style, "fontFamily").map(|s| s.to_string()),
            font_weight: style.get("fontWeight").and_then(|v| v.as_f64()),
            font_size: style.get("fontSize").and_then(|v| v.as_f64()),
            line_height: if val_str(style, "lineHeightUnit") == Some("AUTO") {
                Some(FLineHeight::Auto("AUTO".into()))
            } else {
                style
                    .get("lineHeightPx")
                    .and_then(|v| v.as_f64())
                    .map(FLineHeight::Px)
            },
            letter_spacing: style.get("letterSpacing").and_then(|v| v.as_f64()),
            text_decoration: val_str(style, "textDecoration").map(|s| s.to_string()),
            text_case: val_str(style, "textCase").map(|s| s.to_string()),
            text_align_horizontal: val_str(style, "textAlignHorizontal").map(|s| s.to_string()),
            text_align_vertical: val_str(style, "textAlignVertical").map(|s| s.to_string()),
            width: bounding_box_dim(node, "width"),
        }));
    }

    if typ == "RECTANGLE" {
        let w = bounding_box_dim(node, "width").unwrap_or(0.0);
        let h = bounding_box_dim(node, "height").unwrap_or(0.0);

        // Check for image fills → promote to IMAGE node
        if let Some(fills_arr) = node.get("fills").and_then(|f| f.as_array()) {
            let img_fill = fills_arr
                .iter()
                .find(|f| val_str(f, "type") == Some("IMAGE") && val_bool(f, "visible", true));
            if let Some(img) = img_fill {
                let image_ref = val_str(img, "imageRef").unwrap_or("");
                let src = image_map
                    .get(image_ref)
                    .cloned()
                    .unwrap_or_else(|| format!("figma://image/{image_ref}"));
                let scale_mode = val_str(img, "scaleMode").unwrap_or("FILL");
                return Some(FNode::IMAGE(FImageData {
                    base,
                    width: w,
                    height: h,
                    src,
                    alt: None,
                    object_fit: Some(
                        match scale_mode {
                            "FIT" => "contain",
                            _ => "cover",
                        }
                        .to_string(),
                    ),
                }));
            }
        }

        return Some(FNode::RECTANGLE(FShapeData {
            base,
            width: w,
            height: h,
        }));
    }

    if typ == "ELLIPSE" {
        return Some(FNode::ELLIPSE(FShapeData {
            base,
            width: bounding_box_dim(node, "width").unwrap_or(0.0),
            height: bounding_box_dim(node, "height").unwrap_or(0.0),
        }));
    }

    let vector_types = [
        "VECTOR",
        "LINE",
        "STAR",
        "REGULAR_POLYGON",
        "BOOLEAN_OPERATION",
    ];
    if vector_types.contains(&typ) {
        let svg_path = node
            .get("fillGeometry")
            .and_then(|g| g.as_array())
            .and_then(|a| a.first())
            .and_then(|g| val_str(g, "path"))
            .map(|s| s.to_string());

        let data = FVectorData {
            base,
            width: bounding_box_dim(node, "width").unwrap_or(0.0),
            height: bounding_box_dim(node, "height").unwrap_or(0.0),
            svg_path,
        };
        return match typ {
            "LINE" => Some(FNode::LINE(data)),
            "STAR" => Some(FNode::STAR(data)),
            "REGULAR_POLYGON" => Some(FNode::POLYGON(data)),
            _ => Some(FNode::VECTOR(data)),
        };
    }

    // Fallback: frame-like with children
    if node.get("children").is_some() {
        let children = convert_children(node, image_map);
        return Some(FNode::FRAME(FFrameData {
            base,
            width: bounding_box_dim(node, "width"),
            height: bounding_box_dim(node, "height"),
            layout_mode: None,
            layout_wrap: None,
            primary_axis_align: None,
            counter_axis_align: None,
            primary_axis_sizing: None,
            counter_axis_sizing: None,
            layout_grow: None,
            item_spacing: None,
            counter_axis_spacing: None,
            padding_left: None,
            padding_right: None,
            padding_top: None,
            padding_bottom: None,
            children,
        }));
    }

    None
}

fn convert_children(parent: &Value, image_map: &HashMap<String, String>) -> Vec<FNode> {
    parent
        .get("children")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| convert_node(c, image_map))
                .collect()
        })
        .unwrap_or_default()
}

// ─── Public API ──────────────────────────────────────────────────────

/// Convert a raw Figma REST API file response (`GET /v1/files/:key`) into an FDocument.
pub fn convert_figma_file(
    figma_json: &str,
    image_map: &HashMap<String, String>,
    page_index: usize,
) -> anyhow::Result<FDocument> {
    let root: Value = serde_json::from_str(figma_json)?;
    let doc = root.get("document").unwrap_or(&root);
    let pages = doc.get("children").and_then(|c| c.as_array());

    let page = pages
        .and_then(|p| p.get(page_index).or_else(|| p.first()))
        .ok_or_else(|| anyhow::anyhow!("No pages found in Figma file"))?;

    let children = convert_children(page, image_map);
    let bg = page.get("backgroundColor").map(parse_color);

    Ok(FDocument {
        version: "1.0".to_string(),
        name: val_str(page, "name")
            .or_else(|| val_str(&root, "name"))
            .unwrap_or("Untitled")
            .to_string(),
        width: None,
        height: None,
        background: bg,
        children,
        meta: Some(FDocumentMeta {
            figma_file_key: val_str(&root, "key").map(|s| s.to_string()),
            figma_node_id: None,
            exported_at: Some(chrono::Utc::now().to_rfc3339()),
            generator: Some("figma-api".to_string()),
        }),
    })
}

/// Convert a Figma node response (`GET /v1/files/:key/nodes?ids=...`) into an FDocument.
pub fn convert_figma_node(
    node_json: &str,
    image_map: &HashMap<String, String>,
) -> anyhow::Result<FDocument> {
    let root: Value = serde_json::from_str(node_json)?;
    let nodes_map = root
        .get("nodes")
        .and_then(|n| n.as_object())
        .ok_or_else(|| anyhow::anyhow!("No 'nodes' field in response"))?;

    let (node_id, node_val) = nodes_map
        .iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Empty nodes map"))?;

    let node_doc = node_val
        .get("document")
        .ok_or_else(|| anyhow::anyhow!("No document in node"))?;

    let converted = convert_node(node_doc, image_map);

    Ok(FDocument {
        version: "1.0".to_string(),
        name: val_str(node_doc, "name").unwrap_or("Untitled").to_string(),
        width: bounding_box_dim(node_doc, "width"),
        height: bounding_box_dim(node_doc, "height"),
        children: converted.into_iter().collect(),
        background: None,
        meta: Some(FDocumentMeta {
            figma_file_key: None,
            figma_node_id: Some(node_id.clone()),
            exported_at: Some(chrono::Utc::now().to_rfc3339()),
            generator: Some("figma-api".to_string()),
        }),
    })
}

/// Fetch a Figma file via REST API and convert to FDocument.
/// Runs the HTTP request in Rust (reqwest) for better performance.
pub async fn fetch_and_convert(
    file_key: &str,
    token: &str,
    page_index: usize,
) -> anyhow::Result<FDocument> {
    let client = reqwest::Client::new();

    // Fetch file
    let file_url = format!("https://api.figma.com/v1/files/{file_key}?geometry=paths");
    let file_res = client
        .get(&file_url)
        .header("X-Figma-Token", token)
        .send()
        .await?;

    if !file_res.status().is_success() {
        anyhow::bail!(
            "Figma API error: {} {}",
            file_res.status(),
            file_res.text().await?
        );
    }
    let file_json = file_res.text().await?;

    // Fetch images
    let img_url = format!("https://api.figma.com/v1/files/{file_key}/images");
    let img_res = client
        .get(&img_url)
        .header("X-Figma-Token", token)
        .send()
        .await?;

    let image_map: HashMap<String, String> = if img_res.status().is_success() {
        let img_data: Value = img_res.json().await?;
        img_data
            .get("meta")
            .and_then(|m| m.get("images"))
            .and_then(|i| i.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    convert_figma_file(&file_json, &image_map, page_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_minimal_figma_file() {
        let json = r#"{
            "name": "Test",
            "document": {
                "children": [{
                    "name": "Page 1",
                    "type": "CANVAS",
                    "children": [{
                        "id": "1:2",
                        "name": "Frame",
                        "type": "FRAME",
                        "absoluteBoundingBox": { "x": 0, "y": 0, "width": 200, "height": 100 },
                        "children": []
                    }]
                }]
            }
        }"#;

        let doc = convert_figma_file(json, &HashMap::new(), 0).unwrap();
        assert_eq!(doc.name, "Page 1");
        assert_eq!(doc.children.len(), 1);
        assert_eq!(doc.children[0].base().id, "1:2");
    }

    #[test]
    fn convert_text_node_with_style() {
        let json = r#"{
            "document": {
                "children": [{
                    "name": "Page",
                    "type": "CANVAS",
                    "children": [{
                        "id": "2:1",
                        "name": "Hello",
                        "type": "TEXT",
                        "characters": "Hello World",
                        "style": {
                            "fontFamily": "Inter",
                            "fontSize": 16,
                            "fontWeight": 600,
                            "textAlignHorizontal": "CENTER"
                        },
                        "fills": [{ "type": "SOLID", "color": { "r": 0, "g": 0, "b": 0, "a": 1 } }]
                    }]
                }]
            }
        }"#;

        let doc = convert_figma_file(json, &HashMap::new(), 0).unwrap();
        if let FNode::TEXT(txt) = &doc.children[0] {
            assert_eq!(txt.characters, "Hello World");
            assert_eq!(txt.font_family, Some("Inter".into()));
            assert_eq!(txt.font_size, Some(16.0));
        } else {
            panic!("Expected TEXT node");
        }
    }
}

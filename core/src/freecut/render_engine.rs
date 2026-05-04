//! Frame rendering engine for FreeCut.
//!
//! Renders individual frames or playback sequences using FFmpeg subprocess.
//! All output is written to disk — the frontend receives file paths via events.
//!
//! **Architecture**: The render engine is 100% event-driven. The frontend
//! never decodes media directly. Instead:
//! 1. Frontend calls `render_frame(project, frame_number)`
//! 2. Rust decodes + composes the frame via FFmpeg
//! 3. Rust writes the frame to a temp file (BMP for speed)
//! 4. Rust emits `freecut://frame-rendered` with the file path
//! 5. Frontend loads the image onto a canvas

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

use super::project::{
    ItemType, Project, ProjectResolution, ProjectTimeline, TimelineItem,
};

/// Rendered frame output.
#[derive(Debug, Clone)]
pub struct RenderedFrame {
    pub frame: u32,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}

use super::project::TimelineKeyframe;

/// Returns `true` if `text` contains Khmer (U+1780..U+17FF) or other complex script
/// codepoints that require HarfBuzz shaping and a font with proper OpenType tables.
fn contains_complex_script(text: &str) -> bool {
    text.chars().any(|c| {
        let cp = c as u32;
        // Khmer block U+1780–U+17FF
        (0x1780..=0x17FF).contains(&cp)
        // Thai block U+0E00–U+0E7F
        || (0x0E00..=0x0E7F).contains(&cp)
        // Arabic block U+0600–U+06FF
        || (0x0600..=0x06FF).contains(&cp)
        // Devanagari U+0900–U+097F
        || (0x0900..=0x097F).contains(&cp)
    })
}

/// Resolve the directory containing bundled font files (e.g. `NotoSansKhmer.ttf`).
/// At runtime the binary lives in `desktop/src-tauri/target/…` so we walk up from
/// `env::current_exe()` looking for `core/assets/fonts/`. As a fallback we also
/// try the cargo workspace root relative to `CARGO_MANIFEST_DIR` at compile time.
fn bundled_fonts_dir() -> Option<PathBuf> {
    // 1. Try relative to current exe (production)
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().take(8) {
            let candidate = ancestor.join("core").join("assets").join("fonts");
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }
    // 2. Try CARGO_MANIFEST_DIR (development / tests)
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let candidate = PathBuf::from(manifest).join("assets").join("fonts");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

/// Resolve the effective font name for ASS rendering.
/// When the text contains complex script characters (Khmer, Thai, etc.) and the
/// user hasn't explicitly selected a font, fall back to `Noto Sans Khmer` which
/// ships in `core/assets/fonts/` and renders perfectly with libass/HarfBuzz.
fn resolve_font_name(user_font: Option<&str>, text: &str) -> String {
    match user_font {
        Some(f) if !f.is_empty() => f.to_string(),
        _ if contains_complex_script(text) => "Noto Sans Khmer".to_string(),
        _ => "Arial".to_string(),
    }
}

/// Statically evaluate a keyframe property at a specific absolute frame.
fn eval_keyframe_at(
    prop: &str,
    base_val: f64,
    keyframes: &[TimelineKeyframe],
    item_start_frame: u32,
    current_frame: u32,
) -> f64 {
    if current_frame < item_start_frame {
        return base_val;
    }
    let frame_offset = current_frame - item_start_frame;

    let mut matching: Vec<_> = keyframes.iter().filter(|k| k.property == prop).collect();
    if matching.is_empty() {
        return base_val;
    }
    matching.sort_by_key(|k| k.frame_offset);

    if frame_offset <= matching.first().unwrap().frame_offset {
        return matching.first().unwrap().value;
    }
    if frame_offset >= matching.last().unwrap().frame_offset {
        return matching.last().unwrap().value;
    }

    for w in matching.windows(2) {
        let k0 = w[0];
        let k1 = w[1];
        if frame_offset >= k0.frame_offset && frame_offset < k1.frame_offset {
            let dur = (k1.frame_offset - k0.frame_offset) as f64;
            let ratio = (frame_offset - k0.frame_offset) as f64 / dur;
            return k0.value + (k1.value - k0.value) * ratio;
        }
    }
    base_val
}

/// Dynamically build an FFmpeg expression string for properties evaluated across output time `t`.
fn build_keyframe_expr(
    prop: &str,
    base_val: f64,
    keyframes: &[TimelineKeyframe],
    fps: f64,
    item_start_sec: f64,
) -> String {
    let mut matching: Vec<_> = keyframes.iter().filter(|k| k.property == prop).collect();
    if matching.is_empty() {
        return format!("{:.6}", base_val);
    }
    matching.sort_by_key(|k| k.frame_offset);

    let frame_expr = format!("(t-{:.6})*{}", item_start_sec, fps);

    let mut expr = format!("{:.6}", matching.last().unwrap().value);
    for i in (0..matching.len()).rev() {
        if i == 0 {
            let k = &matching[i];
            expr = format!("if(lt({frame_expr},{}),{:.6},{})", k.frame_offset, k.value, expr);
        } else {
            let k0 = &matching[i - 1];
            let k1 = &matching[i];
            let duration = k1.frame_offset as f64 - k0.frame_offset as f64;
            let lerp = if duration > 0.0 {
                let range = k1.value - k0.value;
                format!(
                    "{:.6}+({:.6}*(({frame_expr}-{})/{}))",
                    k0.value, range, k0.frame_offset, duration
                )
            } else {
                format!("{:.6}", k1.value)
            };
            expr = format!("if(lt({frame_expr},{}),{},{})", k1.frame_offset, lerp, expr);
        }
    }
    expr
}

/// Render a single frame from a project at the given frame number.
///
/// This is the core rendering function. It:
/// 1. Finds all visible items at the target frame
/// 2. Decodes each source at the correct timestamp
/// 3. Composes them using FFmpeg filter_complex
/// 4. Outputs a single frame image at the project resolution
///
/// Returns the path to the rendered frame image.
pub fn render_frame(
    project: &Project,
    frame: u32,
    output_dir: &Path,
    ffmpeg_bin: Option<&str>,
) -> Result<RenderedFrame> {
    let bin = ffmpeg_bin.unwrap_or("ffmpeg");
    let res = &project.metadata;

    std::fs::create_dir_all(output_dir)?;

    let out_path = output_dir.join(format!("frame_{frame:06}.bmp"));

    // Collect items visible at this frame.
    let visible_items = match &project.timeline {
        Some(tl) => collect_visible_items(tl, frame),
        None => vec![],
    };

    if visible_items.is_empty() {
        // No items → render solid background color.
        render_solid_frame(bin, res, &out_path)?;
    } else {
        // Compose visible items via FFmpeg filter_complex.
        render_composed_frame(bin, res, &visible_items, frame, &out_path)?;
    }

    // Verify the output exists and get dimensions.
    if !out_path.exists() {
        bail!(
            "render failed — output file not created: {}",
            out_path.display()
        );
    }

    Ok(RenderedFrame {
        frame,
        path: out_path,
        width: res.width,
        height: res.height,
    })
}

/// Render a solid background frame (no items on timeline).
fn render_solid_frame(ffmpeg_bin: &str, res: &ProjectResolution, out: &Path) -> Result<()> {
    let color = &res.background_color;
    // lavfi color source → single frame
    let status = std::process::Command::new(ffmpeg_bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:s={}x{}:d=0.04", res.width, res.height),
            "-frames:v",
            "1",
        ])
        .arg(out.as_os_str())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .context("failed to run ffmpeg for solid frame")?;

    if !status.success() {
        bail!("ffmpeg solid frame render failed");
    }
    Ok(())
}

/// Render a composed frame with multiple items overlaid.
fn render_composed_frame(
    ffmpeg_bin: &str,
    res: &ProjectResolution,
    items: &[&TimelineItem],
    frame: u32,
    out: &Path,
) -> Result<()> {
    // For items with source media, we decode and overlay.
    // For now, we handle the first video/image item and compose it onto the background.

    let mut cmd = std::process::Command::new(ffmpeg_bin);
    cmd.arg("-y");

    // Background: solid color
    cmd.args([
        "-f",
        "lavfi",
        "-i",
        &format!(
            "color=c={}:s={}x{}:d=0.04",
            res.background_color, res.width, res.height
        ),
    ]);

    let mut input_index = 1u32;
    let mut text_index = 0u32;
    let mut filter_parts: Vec<String> = Vec::new();
    let mut last_label = "[0:v]".to_string();
    let mut temp_files: Vec<PathBuf> = Vec::new();

    for item in items {
        match item.item_type {
            ItemType::Video | ItemType::Image => {
                if let Some(ref src) = item.src {
                    let src_path = Path::new(src);
                    if src_path.exists() {
                        if item.item_type == ItemType::Video {
                            // Calculate seek time within source.
                            let source_fps = item.source_fps.unwrap_or(res.fps as f64);
                            let item_offset = frame.saturating_sub(item.from);
                            let speed = item.speed.unwrap_or(1.0);
                            let source_frame =
                                item.source_start.unwrap_or(0) as f64 + item_offset as f64 * speed;
                            let seek_time = source_frame / source_fps;

                            cmd.args(["-ss", &format!("{seek_time:.4}")]);
                            cmd.args(["-i"]).arg(src_path.as_os_str());
                        } else {
                            cmd.args(["-loop", "1", "-i"]).arg(src_path.as_os_str());
                        }

                        // Get static or keyframe-interpolated transform values.
                        let base_x = item.transform.as_ref().and_then(|t| t.x).unwrap_or(0.0);
                        let base_y = item.transform.as_ref().and_then(|t| t.y).unwrap_or(0.0);
                        let base_op = item.transform.as_ref().and_then(|t| t.opacity).unwrap_or(1.0);
                        let base_sc = item.transform.as_ref().and_then(|t| t.scale).unwrap_or(1.0);

                        let x = eval_keyframe_at("x", base_x, &item.keyframes, item.from, frame);
                        let y = eval_keyframe_at("y", base_y, &item.keyframes, item.from, frame);
                        let mut opacity = eval_keyframe_at("opacity", base_op, &item.keyframes, item.from, frame);
                        let scale = eval_keyframe_at("scale", base_sc, &item.keyframes, item.from, frame);

                        // Apply video fade in/out as opacity modulation for preview.
                        let item_offset = frame.saturating_sub(item.from) as f64;
                        let item_dur = item.duration_in_frames as f64;
                        if let Some(fi) = item.fade_in {
                            if fi > 0.0 && item_offset < fi {
                                opacity *= item_offset / fi;
                            }
                        }
                        if let Some(fo) = item.fade_out {
                            if fo > 0.0 {
                                let fo_start = item_dur - fo;
                                if item_offset > fo_start {
                                    opacity *= ((item_dur - item_offset) / fo).clamp(0.0, 1.0);
                                }
                            }
                        }

                        let base_w = item.transform.as_ref().and_then(|t| t.width).unwrap_or(res.width as f64);
                        let base_h = item.transform.as_ref().and_then(|t| t.height).unwrap_or(res.height as f64);
                        
                        let w = base_w * scale;
                        let h = base_h * scale;

                        let scaled_label = format!("[s{input_index}]");
                        let overlay_label = format!("[o{input_index}]");

                        // Scale source to item dimensions.
                        filter_parts.push(format!(
                            "[{input_index}:v]scale={w:.0}:{h:.0},format=rgba{scaled_label}"
                        ));

                        // Overlay onto the current composition.
                        if opacity < 1.0 {
                            let alpha_label = format!("[a{input_index}]");
                            filter_parts.push(format!(
                                "{scaled_label}colorchannelmixer=aa={opacity:.3}{alpha_label}"
                            ));
                            filter_parts.push(format!(
                                "{last_label}{alpha_label}overlay=x={x:.0}:y={y:.0}{overlay_label}"
                            ));
                        } else {
                            filter_parts.push(format!(
                                "{last_label}{scaled_label}overlay=x={x:.0}:y={y:.0}{overlay_label}"
                            ));
                        }

                        last_label = overlay_label;
                        input_index += 1;
                    }
                }
            }
            ItemType::Text => {
                // Text rendering via subtitles + overlay to guarantee HarfBuzz complex shaping (Khmer/Arabic).
                let text = item.text.as_deref().unwrap_or("");
                let font_size = item.font_size.unwrap_or(60.0) as u32;
                let color_str = item.color.as_deref().unwrap_or("white");
                
                // Convert CSS color to ASS color format (&HAABBGGRR)
                let ass_color = if color_str.starts_with('#') && color_str.len() >= 7 {
                    let r = &color_str[1..3];
                    let g = &color_str[3..5];
                    let b = &color_str[5..7];
                    format!("&H00{}{}{}", b, g, r)
                } else {
                    match color_str.to_lowercase().as_str() {
                        "black" => "&H00000000".to_string(),
                        "red" => "&H000000FF".to_string(),
                        "green" => "&H0000FF00".to_string(),
                        "blue" => "&H00FF0000".to_string(),
                        "yellow" => "&H0000FFFF".to_string(),
                        _ => "&H00FFFFFF".to_string(),
                    }
                };

                let font_name = resolve_font_name(item.font_family.as_deref(), text);
                let fonts_dir_param = bundled_fonts_dir()
                    .map(|d| format!(":fontsdir='{}'", d.to_string_lossy().replace('\\', "/").replace(':', "\\:")))
                    .unwrap_or_default();
                
                let base_x = item.transform.as_ref().and_then(|t| t.x).unwrap_or((res.width / 2) as f64);
                let base_y = item.transform.as_ref().and_then(|t| t.y).unwrap_or((res.height / 2) as f64);
                
                let x = eval_keyframe_at("x", base_x, &item.keyframes, item.from, frame);
                let y = eval_keyframe_at("y", base_y, &item.keyframes, item.from, frame);

                let text_label = format!("[t{text_index}]");
                
                // Write text to an ASS file to prevent Windows CLI from mangling complex unicode (Khmer)
                // Alignment 7 = Top-Left, matching default drawtext behavior.
                let temp_ass = out.parent().unwrap_or_else(|| Path::new("")).join(format!("text_{}_{}.ass", frame, text_index));
                temp_files.push(temp_ass.clone());
                let ass_content = format!(
                    "[Script Info]\n\
                    ScriptType: v4.00+\n\
                    PlayResX: {w}\n\
                    PlayResY: {h}\n\
                    \n\
                    [V4+ Styles]\n\
                    Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
                    Style: Default,{font_name},{font_size},{ass_color},&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,7,0,0,0,1\n\
                    \n\
                    [Events]\n\
                    Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n\
                    Dialogue: 0,0:00:00.00,9:00:00.00,Default,,0,0,0,,{text}\n",
                    w = res.width,
                    h = res.height,
                    font_name = font_name,
                    font_size = font_size,
                    ass_color = ass_color,
                    text = text.replace('\n', "\\N")
                );

                let _ = std::fs::write(&temp_ass, ass_content);
                let escaped_path = temp_ass.to_string_lossy().replace('\\', "/").replace(':', "\\:");

                // Render subtitles on a transparent dummy stream, then overlay with evaluated x/y.
                filter_parts.push(format!(
                    "color=c=black@0.0:s={w}x{h}:r=30:d=1,format=rgba[txtbg{idx}];\
                     [txtbg{idx}]ass='{escaped_path}':alpha=1{fonts_dir}[txtlayer{idx}];\
                     {last_label}[txtlayer{idx}]overlay=x={x:.0}:y={y:.0}{text_label}",
                    w = res.width,
                    h = res.height,
                    idx = text_index,
                    escaped_path = escaped_path,
                    fonts_dir = fonts_dir_param,
                    last_label = last_label,
                    x = x,
                    y = y,
                    text_label = text_label
                ));
                last_label = text_label;
                text_index += 1;
            }
            _ => {
                // Shape, Adjustment, Composition — skip for now.
            }
        }
    }

    if filter_parts.is_empty() {
        // No items with sources → just render the background.
        cmd.args(["-frames:v", "1"]);
        cmd.arg(out.as_os_str());
    } else {
        // Map the final label to output.
        let filter = filter_parts.join(";");
        cmd.args([
            "-filter_complex",
            &format!("{filter};{last_label}null[out]"),
        ]);
        cmd.args(["-map", "[out]", "-frames:v", "1"]);
        cmd.arg(out.as_os_str());
    }

    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());

    let output = cmd
        .output()
        .context("failed to run ffmpeg for composed frame")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffmpeg compose failed: {stderr}");
    }

    for temp in temp_files {
        let _ = std::fs::remove_file(temp);
    }

    Ok(())
}

/// Collect all timeline items visible at the given frame.
fn collect_visible_items(timeline: &ProjectTimeline, frame: u32) -> Vec<&TimelineItem> {
    timeline
        .items
        .iter()
        .filter(|item| {
            let start = item.from;
            let end = item.from + item.duration_in_frames;
            frame >= start && frame < end
        })
        .collect()
}

/// Verify a rendered frame's actual pixel dimensions using FFprobe.
pub fn verify_frame_dimensions(frame_path: &Path, ffprobe_bin: Option<&str>) -> Result<(u32, u32)> {
    let bin = ffprobe_bin.unwrap_or("ffprobe");

    let output = std::process::Command::new(bin)
        .args(["-v", "quiet", "-print_format", "json", "-show_streams"])
        .arg(frame_path.as_os_str())
        .output()
        .context("failed to run ffprobe on rendered frame")?;

    if !output.status.success() {
        bail!("ffprobe failed on frame: {}", frame_path.display());
    }

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let streams = parsed["streams"]
        .as_array()
        .context("no streams in ffprobe output")?;

    for stream in streams {
        if stream["codec_type"].as_str() == Some("video") {
            let w = stream["width"]
                .as_u64()
                .context("no width in video stream")? as u32;
            let h = stream["height"]
                .as_u64()
                .context("no height in video stream")? as u32;
            return Ok((w, h));
        }
    }

    bail!("no video stream found in frame: {}", frame_path.display())
}

/// Generate a synthetic test video (solid color + text) for testing.
///
/// Creates a short video at the specified resolution with a frame counter overlay.
pub fn generate_test_video(
    output_path: &Path,
    width: u32,
    height: u32,
    fps: u32,
    duration_secs: f64,
    ffmpeg_bin: Option<&str>,
) -> Result<()> {
    let bin = ffmpeg_bin.unwrap_or("ffmpeg");

    // Generate a test pattern video with frame counter.
    let status = std::process::Command::new(bin)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc=size={width}x{height}:rate={fps}:duration={duration_secs:.1}"),
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(output_path.as_os_str())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .context("failed to generate test video")?;

    if !status.success() {
        bail!("ffmpeg test video generation failed");
    }

    Ok(())
}

// ─── Full Video Export ─────────────────────────────────────────────────────────

use std::io::{BufRead, BufReader};

use super::project::ExportConfig;

/// Export the entire project to a video file.
///
/// Uses a single FFmpeg invocation with `filter_complex` to compose all
/// timeline items (video, image, text, audio) into the final output.
/// Progress is reported via the `on_progress` callback: `(current_frame, total_frames)`.
pub fn export_video(
    project: &Project,
    config: &ExportConfig,
    ffmpeg_bin: Option<&str>,
    on_progress: impl Fn(u32, u32) + Send,
) -> Result<PathBuf> {
    let bin = ffmpeg_bin.unwrap_or("ffmpeg");
    let res = &project.metadata;
    let fps = res.fps;
    let timeline = project
        .timeline
        .as_ref()
        .context("no timeline — add clips before exporting")?;

    if timeline.items.is_empty() {
        bail!("timeline has no items — add clips before exporting");
    }

    // Calculate total duration from items if project.duration is 0.
    let total_frames = if project.duration > 0 {
        project.duration
    } else {
        timeline
            .items
            .iter()
            .map(|i| i.from + i.duration_in_frames)
            .max()
            .unwrap_or(0)
    };

    if total_frames == 0 {
        bail!("project has no duration — nothing to export");
    }

    let total_duration = total_frames as f64 / fps as f64;
    let output_path = PathBuf::from(&config.output_path);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // ── Build FFmpeg arguments ──────────────────────────────────────────

    let mut args: Vec<String> = vec!["-y".into(), "-progress".into(), "pipe:1".into()];

    // Input 0: solid background for the full duration.
    args.extend([
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        format!(
            "color=c={}:s={}x{}:d={:.6}:r={}",
            res.background_color, res.width, res.height, total_duration, fps
        ),
    ]);

    let mut input_idx = 1u32;
    let mut text_idx = 0u32;
    let mut filter_parts: Vec<String> = Vec::new();
    let mut last_video = "[0:v]".to_string();
    let mut audio_labels: Vec<String> = Vec::new();
    let mut temp_files: Vec<PathBuf> = Vec::new();

    // Sort items by layer order (from position).
    let mut sorted: Vec<&TimelineItem> = timeline.items.iter().collect();
    sorted.sort_by_key(|i| i.from);

    for item in &sorted {
        match item.item_type {
            ItemType::Video | ItemType::Image => {
                let src = match item.src.as_deref() {
                    Some(s) if Path::new(s).exists() => s,
                    _ => continue,
                };

                if item.item_type == ItemType::Image {
                    args.extend(["-loop".into(), "1".into()]);
                }
                args.extend(["-i".into(), src.to_string()]);

                let item_start = item.from as f64 / fps as f64;
                let item_dur = item.duration_in_frames as f64 / fps as f64;
                let speed = item.speed.unwrap_or(1.0);
                let sfps = item.source_fps.unwrap_or(fps as f64);
                let src_start = if item.item_type == ItemType::Image {
                    0.0
                } else {
                    item.source_start.unwrap_or(0) as f64 / sfps
                };

                let base_w = item.transform.as_ref().and_then(|t| t.width).unwrap_or(res.width as f64);
                let base_h = item.transform.as_ref().and_then(|t| t.height).unwrap_or(res.height as f64);
                let base_x = item.transform.as_ref().and_then(|t| t.x).unwrap_or(0.0);
                let base_y = item.transform.as_ref().and_then(|t| t.y).unwrap_or(0.0);
                let opacity = item.transform.as_ref().and_then(|t| t.opacity).unwrap_or(1.0);
                let scale = item.transform.as_ref().and_then(|t| t.scale).unwrap_or(1.0);

                let x_expr = build_keyframe_expr("x", base_x, &item.keyframes, fps as f64, item_start);
                let y_expr = build_keyframe_expr("y", base_y, &item.keyframes, fps as f64, item_start);

                let w = base_w * scale;
                let h = base_h * scale;

                let tr = format!("[vt{input_idx}]");
                let sc = format!("[vs{input_idx}]");
                let ov = format!("[vo{input_idx}]");

                // Trim source.
                filter_parts.push(format!(
                    "[{input_idx}:v]trim=start={src_start:.6}:duration={:.6},setpts=PTS-STARTPTS{tr}",
                    item_dur / speed
                ));

                // Scale + format + video fades.
                let mut scale_chain = format!("{tr}scale={w:.0}:{h:.0},format=rgba");
                if let Some(fi) = item.fade_in {
                    if fi > 0.0 {
                        let fi_dur = fi / fps as f64;
                        scale_chain.push_str(&format!(",fade=t=in:st=0:d={fi_dur:.4}"));
                    }
                }
                if let Some(fo) = item.fade_out {
                    if fo > 0.0 {
                        let fo_dur = fo / fps as f64;
                        let fo_start = (item_dur - fo_dur).max(0.0);
                        scale_chain.push_str(&format!(",fade=t=out:st={fo_start:.4}:d={fo_dur:.4}"));
                    }
                }
                filter_parts.push(format!("{scale_chain}{sc}"));

                // Overlay with enable timing using x/y expressions.
                let end_t = item_start + item_dur;
                if opacity < 1.0 {
                    let al = format!("[va{input_idx}]");
                    filter_parts.push(format!("{sc}colorchannelmixer=aa={opacity:.3}{al}"));
                    filter_parts.push(format!(
                        "{last_video}{al}overlay=x='{x_expr}':y='{y_expr}':enable='between(t,{item_start:.6},{end_t:.6})':eof_action=pass{ov}"
                    ));
                } else {
                    filter_parts.push(format!(
                        "{last_video}{sc}overlay=x='{x_expr}':y='{y_expr}':enable='between(t,{item_start:.6},{end_t:.6})':eof_action=pass{ov}"
                    ));
                }
                last_video = ov;

                // Audio from video sources.
                if item.item_type == ItemType::Video {
                    let vol = item.volume.unwrap_or(1.0);
                    if vol > 0.001 {
                        let delay_ms = (item_start * 1000.0) as i64;
                        let al = format!("[ao{input_idx}]");
                        let mut audio_chain = format!(
                            "[{input_idx}:a]atrim=start={src_start:.6}:duration={:.6},asetpts=PTS-STARTPTS,adelay={delay_ms}|{delay_ms},volume={vol:.3}",
                            item_dur / speed
                        );
                        if let Some(afi) = item.audio_fade_in {
                            if afi > 0.0 {
                                let afi_dur = afi / fps as f64;
                                audio_chain.push_str(&format!(",afade=t=in:st=0:d={afi_dur:.4}"));
                            }
                        }
                        if let Some(afo) = item.audio_fade_out {
                            if afo > 0.0 {
                                let afo_dur = afo / fps as f64;
                                let afo_start = (item_dur - afo_dur).max(0.0);
                                audio_chain.push_str(&format!(",afade=t=out:st={afo_start:.4}:d={afo_dur:.4}"));
                            }
                        }
                        filter_parts.push(format!("{audio_chain}{al}"));
                        audio_labels.push(al);
                    }
                }

                input_idx += 1;
            }
            ItemType::Audio => {
                let src = match item.src.as_deref() {
                    Some(s) if Path::new(s).exists() => s,
                    _ => continue,
                };

                args.extend(["-i".into(), src.to_string()]);

                let item_start = item.from as f64 / fps as f64;
                let item_dur = item.duration_in_frames as f64 / fps as f64;
                let sfps = item.source_fps.unwrap_or(fps as f64);
                let src_start = item.source_start.unwrap_or(0) as f64 / sfps;
                let vol = item.volume.unwrap_or(1.0);
                let delay_ms = (item_start * 1000.0) as i64;

                let al = format!("[ao{input_idx}]");
                let mut audio_chain = format!(
                    "[{input_idx}:a]atrim=start={src_start:.6}:duration={item_dur:.6},asetpts=PTS-STARTPTS,adelay={delay_ms}|{delay_ms},volume={vol:.3}"
                );
                if let Some(afi) = item.audio_fade_in {
                    if afi > 0.0 {
                        let afi_dur = afi / fps as f64;
                        audio_chain.push_str(&format!(",afade=t=in:st=0:d={afi_dur:.4}"));
                    }
                }
                if let Some(afo) = item.audio_fade_out {
                    if afo > 0.0 {
                        let afo_dur = afo / fps as f64;
                        let afo_start = (item_dur - afo_dur).max(0.0);
                        audio_chain.push_str(&format!(",afade=t=out:st={afo_start:.4}:d={afo_dur:.4}"));
                    }
                }
                filter_parts.push(format!("{audio_chain}{al}"));
                audio_labels.push(al);
                input_idx += 1;
            }
            ItemType::Text => {
                let text = item.text.as_deref().unwrap_or("");
                let fsize = item.font_size.unwrap_or(60.0) as u32;
                let color_str = item.color.as_deref().unwrap_or("white");
                
                // Convert CSS color to ASS color format (&HAABBGGRR)
                let ass_color = if color_str.starts_with('#') && color_str.len() >= 7 {
                    let r = &color_str[1..3];
                    let g = &color_str[3..5];
                    let b = &color_str[5..7];
                    format!("&H00{}{}{}", b, g, r)
                } else {
                    match color_str.to_lowercase().as_str() {
                        "black" => "&H00000000".to_string(),
                        "red" => "&H000000FF".to_string(),
                        "green" => "&H0000FF00".to_string(),
                        "blue" => "&H00FF0000".to_string(),
                        "yellow" => "&H0000FFFF".to_string(),
                        _ => "&H00FFFFFF".to_string(),
                    }
                };

                let font_name = resolve_font_name(item.font_family.as_deref(), text);
                let fonts_dir_param = bundled_fonts_dir()
                    .map(|d| format!(":fontsdir='{}'", d.to_string_lossy().replace('\\', "/").replace(':', "\\:")))
                    .unwrap_or_default();
                
                let base_x = item.transform.as_ref().and_then(|t| t.x).unwrap_or((res.width / 2) as f64);
                let base_y = item.transform.as_ref().and_then(|t| t.y).unwrap_or((res.height / 2) as f64);
                
                let item_start = item.from as f64 / fps as f64;
                let x_expr = build_keyframe_expr("x", base_x, &item.keyframes, fps as f64, item_start);
                let y_expr = build_keyframe_expr("y", base_y, &item.keyframes, fps as f64, item_start);

                let t1 = (item.from + item.duration_in_frames) as f64 / fps as f64;

                let temp_ass = output_path.parent().unwrap_or_else(|| Path::new("")).join(format!("export_text_{}_{}.ass", text_idx, input_idx));
                temp_files.push(temp_ass.clone());
                let ass_content = format!(
                    "[Script Info]\n\
                    ScriptType: v4.00+\n\
                    PlayResX: {w}\n\
                    PlayResY: {h}\n\
                    \n\
                    [V4+ Styles]\n\
                    Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
                    Style: Default,{font_name},{font_size},{ass_color},&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,7,0,0,0,1\n\
                    \n\
                    [Events]\n\
                    Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n\
                    Dialogue: 0,0:00:00.00,9:00:00.00,Default,,0,0,0,,{text}\n",
                    w = res.width,
                    h = res.height,
                    font_name = font_name,
                    font_size = fsize,
                    ass_color = ass_color,
                    text = text.replace('\n', "\\N")
                );

                let _ = std::fs::write(&temp_ass, ass_content);
                let escaped_path = temp_ass.to_string_lossy().replace('\\', "/").replace(':', "\\:");

                let lbl = format!("[txt{text_idx}]");
                
                // Render subtitles on a transparent dummy stream, then overlay with evaluated x/y expressions.
                // We use enable='between(t,...)' on the overlay filter.
                filter_parts.push(format!(
                    "color=c=black@0.0:s={w}x{h}:r={fps}:d={dur:.6},format=rgba[txtbg{idx}];\
                     [txtbg{idx}]ass='{escaped_path}':alpha=1{fonts_dir}[txtlayer{idx}];\
                     {last_video}[txtlayer{idx}]overlay=x='{x_expr}':y='{y_expr}':enable='between(t,{item_start:.6},{t1:.6})'{lbl}",
                    w = res.width,
                    h = res.height,
                    fps = fps,
                    dur = total_duration,
                    idx = text_idx,
                    escaped_path = escaped_path,
                    fonts_dir = fonts_dir_param,
                    last_video = last_video,
                    x_expr = x_expr,
                    y_expr = y_expr,
                    item_start = item_start,
                    t1 = t1,
                    lbl = lbl
                ));
                last_video = lbl;
                text_idx += 1;
            }
            _ => {}
        }
    }

    // Finalize filter_complex.
    let has_audio = !audio_labels.is_empty();
    let mut filter = filter_parts.join(";");
    filter.push_str(&format!(";{last_video}null[vout]"));

    if has_audio {
        let labels: String = audio_labels.iter().map(|l| l.as_str()).collect::<String>();
        filter.push_str(&format!(
            ";{labels}amix=inputs={}:duration=longest[aout]",
            audio_labels.len()
        ));
    }

    args.extend(["-filter_complex".into(), filter]);
    args.extend(["-map".into(), "[vout]".into()]);
    if has_audio {
        args.extend(["-map".into(), "[aout]".into()]);
    }

    // Video encoding settings.
    let encoder = if config.hw_accel.is_some() {
        config.hw_accel.as_deref().unwrap_or("libx264")
    } else {
        match config.codec.as_str() {
            "h265" | "hevc" => "libx265",
            "vp9" => "libvpx-vp9",
            _ => "libx264",
        }
    };

    let (preset, crf) = match config.quality.as_str() {
        "low" => ("veryfast", "28"),
        "medium" => ("medium", "23"),
        "ultra" => ("slow", "16"),
        _ => ("medium", "20"), // "high" default
    };

    args.extend([
        "-c:v".into(),
        encoder.to_string(),
        "-preset".into(),
        preset.to_string(),
        "-crf".into(),
        crf.to_string(),
        "-pix_fmt".into(),
        "yuv420p".to_string(),
        "-r".into(),
        fps.to_string(),
    ]);

    if has_audio {
        args.extend([
            "-c:a".into(),
            "aac".to_string(),
            "-b:a".into(),
            "192k".to_string(),
        ]);
    }

    args.push(config.output_path.clone());

    // ── Run FFmpeg with progress tracking ────────────────────────────────

    let mut child = std::process::Command::new(bin)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn ffmpeg for export")?;

    // Parse progress from stdout (-progress pipe:1 emits key=value lines).
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(val) = line.strip_prefix("frame=") {
                if let Ok(frame) = val.trim().parse::<u32>() {
                    on_progress(frame, total_frames);
                }
            }
        }
    }

    let output = child
        .wait_with_output()
        .context("ffmpeg export process failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        for temp in temp_files {
            let _ = std::fs::remove_file(temp);
        }

        // Retry without audio if audio extraction failed.
        if has_audio && stderr.contains("does not contain any stream") {
            return export_video_no_audio(project, config, ffmpeg_bin, on_progress);
        }
        bail!(
            "export failed: {}",
            stderr.chars().take(500).collect::<String>()
        );
    }

    for temp in temp_files {
        let _ = std::fs::remove_file(temp);
    }

    Ok(output_path)
}

/// Fallback: export video without audio (when sources lack audio streams).
fn export_video_no_audio(
    project: &Project,
    config: &ExportConfig,
    ffmpeg_bin: Option<&str>,
    on_progress: impl Fn(u32, u32) + Send,
) -> Result<PathBuf> {
    let bin = ffmpeg_bin.unwrap_or("ffmpeg");
    let res = &project.metadata;
    let fps = res.fps;
    let timeline = project.timeline.as_ref().context("no timeline")?;
    let total_frames = if project.duration > 0 {
        project.duration
    } else {
        timeline
            .items
            .iter()
            .map(|i| i.from + i.duration_in_frames)
            .max()
            .unwrap_or(0)
    };
    let total_duration = total_frames as f64 / fps as f64;
    let output_path = PathBuf::from(&config.output_path);

    let mut args: Vec<String> = vec!["-y".into(), "-progress".into(), "pipe:1".into()];

    // Background.
    args.extend([
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        format!(
            "color=c={}:s={}x{}:d={:.6}:r={}",
            res.background_color, res.width, res.height, total_duration, fps
        ),
    ]);

    let mut input_idx = 1u32;
    let mut text_idx = 0u32;
    let mut filter_parts: Vec<String> = Vec::new();
    let mut last_video = "[0:v]".to_string();
    let mut temp_files: Vec<PathBuf> = Vec::new();

    let mut sorted: Vec<&TimelineItem> = timeline.items.iter().collect();
    sorted.sort_by_key(|i| i.from);

    for item in &sorted {
        match item.item_type {
            ItemType::Video | ItemType::Image => {
                let src = match item.src.as_deref() {
                    Some(s) if Path::new(s).exists() => s,
                    _ => continue,
                };
                if item.item_type == ItemType::Image {
                    args.extend(["-loop".into(), "1".into()]);
                }
                args.extend(["-i".into(), src.to_string()]);

                let item_start = item.from as f64 / fps as f64;
                let item_dur = item.duration_in_frames as f64 / fps as f64;
                let speed = item.speed.unwrap_or(1.0);
                let sfps = item.source_fps.unwrap_or(fps as f64);
                let src_start = if item.item_type == ItemType::Image {
                    0.0
                } else {
                    item.source_start.unwrap_or(0) as f64 / sfps
                };
                let base_w = item.transform.as_ref().and_then(|t| t.width).unwrap_or(res.width as f64);
                let base_h = item.transform.as_ref().and_then(|t| t.height).unwrap_or(res.height as f64);
                let base_x = item.transform.as_ref().and_then(|t| t.x).unwrap_or(0.0);
                let base_y = item.transform.as_ref().and_then(|t| t.y).unwrap_or(0.0);
                let opacity = item.transform.as_ref().and_then(|t| t.opacity).unwrap_or(1.0);
                let scale = item.transform.as_ref().and_then(|t| t.scale).unwrap_or(1.0);

                let x_expr = build_keyframe_expr("x", base_x, &item.keyframes, fps as f64, item_start);
                let y_expr = build_keyframe_expr("y", base_y, &item.keyframes, fps as f64, item_start);

                let w = base_w * scale;
                let h = base_h * scale;

                let tr = format!("[vt{input_idx}]");
                let sc = format!("[vs{input_idx}]");
                let ov = format!("[vo{input_idx}]");
                let end_t = item_start + item_dur;

                filter_parts.push(format!(
                    "[{input_idx}:v]trim=start={src_start:.6}:duration={:.6},setpts=PTS-STARTPTS{tr}",
                    item_dur / speed
                ));
                let mut scale_chain = format!("{tr}scale={w:.0}:{h:.0},format=rgba");
                if let Some(fi) = item.fade_in {
                    if fi > 0.0 {
                        let fi_dur = fi / fps as f64;
                        scale_chain.push_str(&format!(",fade=t=in:st=0:d={fi_dur:.4}"));
                    }
                }
                if let Some(fo) = item.fade_out {
                    if fo > 0.0 {
                        let fo_dur = fo / fps as f64;
                        let fo_start = (item_dur - fo_dur).max(0.0);
                        scale_chain.push_str(&format!(",fade=t=out:st={fo_start:.4}:d={fo_dur:.4}"));
                    }
                }
                filter_parts.push(format!("{scale_chain}{sc}"));
                if opacity < 1.0 {
                    let al = format!("[va{input_idx}]");
                    filter_parts.push(format!("{sc}colorchannelmixer=aa={opacity:.3}{al}"));
                    filter_parts.push(format!(
                        "{last_video}{al}overlay=x='{x_expr}':y='{y_expr}':enable='between(t,{item_start:.6},{end_t:.6})':eof_action=pass{ov}"
                    ));
                } else {
                    filter_parts.push(format!(
                        "{last_video}{sc}overlay=x='{x_expr}':y='{y_expr}':enable='between(t,{item_start:.6},{end_t:.6})':eof_action=pass{ov}"
                    ));
                }
                last_video = ov;
                input_idx += 1;
            }
            ItemType::Text => {
                let text = item.text.as_deref().unwrap_or("");
                let fsize = item.font_size.unwrap_or(60.0) as u32;
                let color_str = item.color.as_deref().unwrap_or("white");
                
                // Convert CSS color to ASS color format (&HAABBGGRR)
                let ass_color = if color_str.starts_with('#') && color_str.len() >= 7 {
                    let r = &color_str[1..3];
                    let g = &color_str[3..5];
                    let b = &color_str[5..7];
                    format!("&H00{}{}{}", b, g, r)
                } else {
                    match color_str.to_lowercase().as_str() {
                        "black" => "&H00000000".to_string(),
                        "red" => "&H000000FF".to_string(),
                        "green" => "&H0000FF00".to_string(),
                        "blue" => "&H00FF0000".to_string(),
                        "yellow" => "&H0000FFFF".to_string(),
                        _ => "&H00FFFFFF".to_string(),
                    }
                };

                let font_name = resolve_font_name(item.font_family.as_deref(), text);
                let fonts_dir_param = bundled_fonts_dir()
                    .map(|d| format!(":fontsdir='{}'", d.to_string_lossy().replace('\\', "/").replace(':', "\\:")))
                    .unwrap_or_default();
                
                let base_x = item.transform.as_ref().and_then(|t| t.x).unwrap_or((res.width / 2) as f64);
                let base_y = item.transform.as_ref().and_then(|t| t.y).unwrap_or((res.height / 2) as f64);
                
                let item_start = item.from as f64 / fps as f64;
                let x_expr = build_keyframe_expr("x", base_x, &item.keyframes, fps as f64, item_start);
                let y_expr = build_keyframe_expr("y", base_y, &item.keyframes, fps as f64, item_start);

                let t1 = (item.from + item.duration_in_frames) as f64 / fps as f64;

                let temp_ass = output_path.parent().unwrap_or_else(|| Path::new("")).join(format!("export_noaudio_text_{}_{}.ass", text_idx, input_idx));
                temp_files.push(temp_ass.clone());
                let ass_content = format!(
                    "[Script Info]\n\
                    ScriptType: v4.00+\n\
                    PlayResX: {w}\n\
                    PlayResY: {h}\n\
                    \n\
                    [V4+ Styles]\n\
                    Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
                    Style: Default,{font_name},{font_size},{ass_color},&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,7,0,0,0,1\n\
                    \n\
                    [Events]\n\
                    Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n\
                    Dialogue: 0,0:00:00.00,9:00:00.00,Default,,0,0,0,,{text}\n",
                    w = res.width,
                    h = res.height,
                    font_name = font_name,
                    font_size = fsize,
                    ass_color = ass_color,
                    text = text.replace('\n', "\\N")
                );

                let _ = std::fs::write(&temp_ass, ass_content);
                let escaped_path = temp_ass.to_string_lossy().replace('\\', "/").replace(':', "\\:");

                let lbl = format!("[txt{text_idx}]");
                
                // Render subtitles on a transparent dummy stream, then overlay with evaluated x/y expressions.
                // We use enable='between(t,...)' on the overlay filter.
                filter_parts.push(format!(
                    "color=c=black@0.0:s={w}x{h}:r={fps}:d={dur:.6},format=rgba[txtbg{idx}];\
                     [txtbg{idx}]ass='{escaped_path}':alpha=1{fonts_dir}[txtlayer{idx}];\
                     {last_video}[txtlayer{idx}]overlay=x='{x_expr}':y='{y_expr}':enable='between(t,{item_start:.6},{t1:.6})'{lbl}",
                    w = res.width,
                    h = res.height,
                    fps = fps,
                    dur = total_duration,
                    idx = text_idx,
                    escaped_path = escaped_path,
                    fonts_dir = fonts_dir_param,
                    last_video = last_video,
                    x_expr = x_expr,
                    y_expr = y_expr,
                    item_start = item_start,
                    t1 = t1,
                    lbl = lbl
                ));
                last_video = lbl;
                text_idx += 1;
            }
            _ => {}
        }
    }

    let mut filter = filter_parts.join(";");
    filter.push_str(&format!(";{last_video}null[vout]"));

    let encoder = if config.hw_accel.is_some() {
        config.hw_accel.as_deref().unwrap_or("libx264")
    } else {
        "libx264"
    };
    let (preset, crf) = match config.quality.as_str() {
        "low" => ("veryfast", "28"),
        "medium" => ("medium", "23"),
        "ultra" => ("slow", "16"),
        _ => ("medium", "20"),
    };

    args.extend(["-filter_complex".into(), filter]);
    args.extend(["-map".into(), "[vout]".into()]);
    args.extend([
        "-c:v".into(),
        encoder.to_string(),
        "-preset".into(),
        preset.to_string(),
        "-crf".into(),
        crf.to_string(),
        "-pix_fmt".into(),
        "yuv420p".to_string(),
        "-r".into(),
        fps.to_string(),
        "-an".into(),
    ]);
    args.push(config.output_path.clone());

    let mut child = std::process::Command::new(bin)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn ffmpeg for export (no-audio fallback)")?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(val) = line.strip_prefix("frame=") {
                if let Ok(frame) = val.trim().parse::<u32>() {
                    on_progress(frame, total_frames);
                }
            }
        }
    }

    let output = child
        .wait_with_output()
        .context("ffmpeg no-audio export failed")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        for temp in &temp_files {
            let _ = std::fs::remove_file(temp);
        }
        bail!(
            "export failed (no-audio fallback): {}",
            stderr.chars().take(500).collect::<String>()
        );
    }

    for temp in temp_files {
        let _ = std::fs::remove_file(temp);
    }

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freecut::project::{
        ItemType, Project, ProjectResolution, ProjectTimeline, TimelineItem, Track,
        TransformProperties,
    };
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_project(width: u32, height: u32, fps: u32) -> Project {
        Project {
            id: "test-render".to_string(),
            name: "Render Test".to_string(),
            description: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            duration: fps * 5, // 5 seconds
            schema_version: 1,
            metadata: ProjectResolution {
                width,
                height,
                fps,
                background_color: "#1a1a2e".to_string(),
            },
            timeline: None,
            dubbing: None,
        }
    }

    // ─── 1080p Solid Frame ──────────────────────────────────────────────

    #[test]
    fn render_1080p_solid_frame() {
        let tmp = TempDir::new().unwrap();
        let project = make_project(1920, 1080, 30);

        let result = render_frame(&project, 0, tmp.path(), None).unwrap();

        assert_eq!(result.width, 1920, "rendered width must be 1920");
        assert_eq!(result.height, 1080, "rendered height must be 1080");
        assert!(result.path.exists(), "frame file must exist");

        // Verify actual pixel dimensions via FFprobe.
        let (actual_w, actual_h) = verify_frame_dimensions(&result.path, None).unwrap();
        assert_eq!(actual_w, 1920, "actual pixel width must be 1920");
        assert_eq!(actual_h, 1080, "actual pixel height must be 1080");

        // Verify file size is reasonable for a 1080p BMP (~6MB uncompressed).
        let file_size = std::fs::metadata(&result.path).unwrap().len();
        assert!(
            file_size > 100_000,
            "1080p frame should be > 100KB, got {file_size}"
        );

        println!(
            "✅ 1080p solid frame: {}x{} @ {} bytes ({})",
            actual_w,
            actual_h,
            file_size,
            result.path.display()
        );
    }

    // ─── 1080p with Test Video ──────────────────────────────────────────

    #[test]
    fn render_1080p_with_video_item() {
        let tmp = TempDir::new().unwrap();

        // Generate a synthetic 1080p test video.
        let test_video = tmp.path().join("test_1080p.mp4");
        generate_test_video(&test_video, 1920, 1080, 30, 2.0, None).unwrap();
        assert!(test_video.exists(), "test video must be created");

        // Create project with a video item on the timeline.
        let mut project = make_project(1920, 1080, 30);
        project.timeline = Some(ProjectTimeline {
            tracks: vec![Track {
                id: "v1".to_string(),
                name: "Video 1".to_string(),
                kind: Some(crate::freecut::project::TrackKind::Video),
                height: 48.0,
                locked: false,
                visible: true,
                muted: false,
                solo: false,
                volume: None,
                color: None,
                order: 0,
                parent_track_id: None,
                is_group: false,
                is_collapsed: false,
            }],
            items: vec![TimelineItem {
                id: "item-1".to_string(),
                track_id: "v1".to_string(),
                from: 0,
                duration_in_frames: 60,
                label: "test_1080p.mp4".to_string(),
                item_type: ItemType::Video,
                media_id: None,
                origin_id: None,
                linked_group_id: None,
                composition_id: None,
                trim_start: None,
                trim_end: None,
                source_start: Some(0),
                source_end: Some(60),
                source_duration: Some(60),
                source_fps: Some(30.0),
                speed: Some(1.0),
                transform: Some(TransformProperties {
                    x: Some(0.0),
                    y: Some(0.0),
                    width: Some(1920.0),
                    height: Some(1080.0),
                    scale: Some(1.0),
                    rotation: None,
                    opacity: Some(1.0),
                    corner_radius: None,
                    aspect_ratio_locked: None,
                }),
                volume: None,
                audio_fade_in: None,
                audio_fade_out: None,
                fade_in: None,
                fade_out: None,
                effects: vec![],
                blend_mode: None,
                keyframes: vec![],
                src: Some(test_video.to_string_lossy().to_string()),
                thumbnail_url: None,
                source_width: Some(1920),
                source_height: Some(1080),
                waveform_data: None,
                text: None,
                font_size: None,
                font_family: None,
                color: None,
                text_align: None,
                shape_type: None,
                fill_color: None,
                stroke_color: None,
                stroke_width: None,
                composition_width: None,
                composition_height: None,
            }],
            ..Default::default()
        });

        // Render frame 15 (0.5 seconds in).
        let result = render_frame(&project, 15, tmp.path(), None).unwrap();

        assert_eq!(result.width, 1920);
        assert_eq!(result.height, 1080);
        assert!(result.path.exists());

        let (actual_w, actual_h) = verify_frame_dimensions(&result.path, None).unwrap();
        assert_eq!(actual_w, 1920, "composed frame width must be 1920");
        assert_eq!(actual_h, 1080, "composed frame height must be 1080");

        let file_size = std::fs::metadata(&result.path).unwrap().len();
        assert!(
            file_size > 100_000,
            "composed 1080p frame should be > 100KB, got {file_size}"
        );

        println!(
            "✅ 1080p composed frame (video overlay): {}x{} @ {} bytes",
            actual_w, actual_h, file_size
        );
    }

    // ─── 1080p with Text Overlay ────────────────────────────────────────

    #[test]
    fn render_1080p_with_text_overlay() {
        let tmp = TempDir::new().unwrap();

        let mut project = make_project(1920, 1080, 30);
        project.timeline = Some(ProjectTimeline {
            tracks: vec![Track {
                id: "t1".to_string(),
                name: "Text 1".to_string(),
                kind: Some(crate::freecut::project::TrackKind::Video),
                height: 48.0,
                locked: false,
                visible: true,
                muted: false,
                solo: false,
                volume: None,
                color: None,
                order: 0,
                parent_track_id: None,
                is_group: false,
                is_collapsed: false,
            }],
            items: vec![TimelineItem {
                id: "text-1".to_string(),
                track_id: "t1".to_string(),
                from: 0,
                duration_in_frames: 90,
                label: "Title".to_string(),
                item_type: ItemType::Text,
                media_id: None,
                origin_id: None,
                linked_group_id: None,
                composition_id: None,
                trim_start: None,
                trim_end: None,
                source_start: None,
                source_end: None,
                source_duration: None,
                source_fps: None,
                speed: None,
                transform: Some(TransformProperties {
                    x: Some(700.0),
                    y: Some(480.0),
                    width: None,
                    height: None,
                    scale: Some(1.0),
                    rotation: None,
                    opacity: Some(1.0),
                    corner_radius: None,
                    aspect_ratio_locked: None,
                }),
                volume: None,
                audio_fade_in: None,
                audio_fade_out: None,
                fade_in: None,
                fade_out: None,
                effects: vec![],
                blend_mode: None,
                keyframes: vec![],
                src: None,
                thumbnail_url: None,
                source_width: None,
                source_height: None,
                waveform_data: None,
                text: Some("FreeCut 1080p".to_string()),
                font_size: Some(72.0),
                font_family: Some("sans-serif".to_string()),
                color: Some("white".to_string()),
                text_align: None,
                shape_type: None,
                fill_color: None,
                stroke_color: None,
                stroke_width: None,
                composition_width: None,
                composition_height: None,
            }],
            ..Default::default()
        });

        let result = render_frame(&project, 0, tmp.path(), None).unwrap();

        assert_eq!(result.width, 1920);
        assert_eq!(result.height, 1080);

        let (actual_w, actual_h) = verify_frame_dimensions(&result.path, None).unwrap();
        assert_eq!(actual_w, 1920, "text overlay frame width must be 1920");
        assert_eq!(actual_h, 1080, "text overlay frame height must be 1080");

        println!(
            "✅ 1080p text overlay: {}x{} — file: {}",
            actual_w,
            actual_h,
            result.path.display()
        );
    }

    // ─── Multiple Resolutions ───────────────────────────────────────────

    #[test]
    fn render_multiple_resolutions() {
        let tmp = TempDir::new().unwrap();

        let resolutions = [
            (1920, 1080, "1080p"),
            (3840, 2160, "4K"),
            (1280, 720, "720p"),
            (640, 360, "360p"),
        ];

        for (w, h, label) in &resolutions {
            let project = make_project(*w, *h, 30);
            let out_dir = tmp.path().join(label);

            let result = render_frame(&project, 0, &out_dir, None).unwrap();

            assert_eq!(result.width, *w, "{label} width mismatch");
            assert_eq!(result.height, *h, "{label} height mismatch");

            let (actual_w, actual_h) = verify_frame_dimensions(&result.path, None).unwrap();
            assert_eq!(actual_w, *w, "{label} actual pixel width mismatch");
            assert_eq!(actual_h, *h, "{label} actual pixel height mismatch");

            println!("✅ {label}: {actual_w}x{actual_h}");
        }
    }

    // ─── Frame at Different Positions ───────────────────────────────────

    #[test]
    fn render_frames_at_different_positions() {
        let tmp = TempDir::new().unwrap();

        // Generate test video.
        let test_video = tmp.path().join("timeline_test.mp4");
        generate_test_video(&test_video, 1920, 1080, 30, 3.0, None).unwrap();

        let mut project = make_project(1920, 1080, 30);
        project.timeline = Some(ProjectTimeline {
            tracks: vec![Track {
                id: "v1".to_string(),
                name: "Video 1".to_string(),
                kind: None,
                height: 48.0,
                locked: false,
                visible: true,
                muted: false,
                solo: false,
                volume: None,
                color: None,
                order: 0,
                parent_track_id: None,
                is_group: false,
                is_collapsed: false,
            }],
            items: vec![TimelineItem {
                id: "v-item".to_string(),
                track_id: "v1".to_string(),
                from: 0,
                duration_in_frames: 90,
                label: "clip".to_string(),
                item_type: ItemType::Video,
                media_id: None,
                origin_id: None,
                linked_group_id: None,
                composition_id: None,
                trim_start: None,
                trim_end: None,
                source_start: Some(0),
                source_end: Some(90),
                source_duration: Some(90),
                source_fps: Some(30.0),
                speed: Some(1.0),
                transform: None,
                volume: None,
                audio_fade_in: None,
                audio_fade_out: None,
                fade_in: None,
                fade_out: None,
                effects: vec![],
                blend_mode: None,
                keyframes: vec![],
                src: Some(test_video.to_string_lossy().to_string()),
                thumbnail_url: None,
                source_width: Some(1920),
                source_height: Some(1080),
                waveform_data: None,
                text: None,
                font_size: None,
                font_family: None,
                color: None,
                text_align: None,
                shape_type: None,
                fill_color: None,
                stroke_color: None,
                stroke_width: None,
                composition_width: None,
                composition_height: None,
            }],
            ..Default::default()
        });

        // Render at frame 0, 15, 45, 89
        for frame in [0, 15, 45, 89] {
            let out_dir = tmp.path().join(format!("frame_{frame}"));
            let result = render_frame(&project, frame, &out_dir, None).unwrap();

            assert_eq!(result.frame, frame);
            let (w, h) = verify_frame_dimensions(&result.path, None).unwrap();
            assert_eq!(w, 1920);
            assert_eq!(h, 1080);
            println!("✅ Frame {frame}: {w}x{h}");
        }
    }

    // ─── Khmer Text Export (E2E) ────────────────────────────────────────

    #[test]
    fn export_khmer_text_video() {
        use crate::freecut::project::ExportConfig;

        let tmp = TempDir::new().unwrap();
        let output_mp4 = tmp.path().join("khmer-export.mp4");

        let mut project = make_project(1280, 720, 30);
        project.duration = 60; // 2 seconds at 30fps
        project.timeline = Some(ProjectTimeline {
            tracks: vec![Track {
                id: "t1".to_string(),
                name: "Khmer Text".to_string(),
                kind: Some(crate::freecut::project::TrackKind::Video),
                height: 48.0,
                locked: false,
                visible: true,
                muted: false,
                solo: false,
                volume: None,
                color: None,
                order: 0,
                parent_track_id: None,
                is_group: false,
                is_collapsed: false,
            }],
            items: vec![TimelineItem {
                id: "khmer-text".to_string(),
                track_id: "t1".to_string(),
                from: 0,
                duration_in_frames: 60,
                label: "Khmer".to_string(),
                item_type: ItemType::Text,
                media_id: None,
                origin_id: None,
                linked_group_id: None,
                composition_id: None,
                trim_start: None,
                trim_end: None,
                source_start: None,
                source_end: None,
                source_duration: None,
                source_fps: None,
                speed: None,
                transform: Some(TransformProperties {
                    x: Some(100.0),
                    y: Some(300.0),
                    width: None,
                    height: None,
                    scale: Some(1.0),
                    rotation: None,
                    opacity: Some(1.0),
                    corner_radius: None,
                    aspect_ratio_locked: None,
                }),
                volume: None,
                audio_fade_in: None,
                audio_fade_out: None,
                fade_in: None,
                fade_out: None,
                effects: vec![],
                blend_mode: None,
                keyframes: vec![],
                src: None,
                thumbnail_url: None,
                source_width: None,
                source_height: None,
                waveform_data: None,
                text: Some("\u{179F}\u{17BD}\u{179F}\u{17D2}\u{178F}\u{17B8} \u{1781}\u{17D2}\u{1798}\u{17C2}\u{179A}".to_string()),
                font_size: Some(72.0),
                font_family: None, // should auto-select Noto Sans Khmer
                color: Some("white".to_string()),
                text_align: None,
                shape_type: None,
                fill_color: None,
                stroke_color: None,
                stroke_width: None,
                composition_width: None,
                composition_height: None,
            }],
            ..Default::default()
        });

        let config = ExportConfig {
            output_path: output_mp4.to_string_lossy().to_string(),
            codec: "libx264".to_string(),
            width: 1280,
            height: 720,
            fps: 30,
            quality: "high".to_string(),
            hw_accel: None,
        };

        let result = export_video(&project, &config, None, |_cur, _total| {});
        assert!(result.is_ok(), "export_video failed: {:?}", result.err());

        let path = result.unwrap();
        assert!(path.exists(), "exported MP4 must exist");

        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size > 1000, "exported MP4 too small ({size} bytes)");

        println!("✅ Khmer export: {} ({} bytes)", path.display(), size);
    }
}

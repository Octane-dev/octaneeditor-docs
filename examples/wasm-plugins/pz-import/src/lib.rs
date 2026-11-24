//! Panzoid `.pz` importer plugin (WASM).
//!
//! A `.pz` is gzip(tar{meta, project(JSON), asset blobs, zoidium.json}). The
//! `project` is a Panzoid Clipmaker 3D scene. This v1 reads the real container +
//! JSON and brings the **text content** in as editable text layers on a comp
//! sized to the sequence. Panzoid positions are 3D world units (no camera here),
//! so 2D placement is approximated (stacked + nudged by world Y); styling/3D look
//! are not reproduced yet.
//!
//! Build: `cargo build --target wasm32-unknown-unknown --release`

use octane_plugin_sdk as octane;
use std::io::Read;
use serde_json::Value;

#[no_mangle]
pub extern "C" fn octane_register() {
    octane::register_importer("Panzoid Project (.pz)", &["pz"], "import_pz");
}

#[no_mangle]
pub extern "C" fn import_pz() {
    let bytes = octane::input_bytes();
    match run(&bytes) {
        Ok(msg) => octane::log(&msg),
        Err(e)  => octane::log(&format!("pz import FAILED: {e}")),
    }
}

struct TextObj { text: String, x: f32, y: f32, size: f32, font: String, color: [f32; 4],
                 effects: Vec<MappedFx>, scale: [f32; 2], rot: f32,
                 /// Opacity keyframes `(frame, value 0..1)` inherited from the clip (empty = static).
                 opacity_keys: Vec<(f32, f32)> }

/// A mapped effect. `Native` = a built-in effect (by id) with indexed params;
/// `Community` = a Panzoid community effect forwarded by name (the host resolves a
/// zoidium port, else the built-in `fallback`), with params fed by name.
#[derive(Clone)]
enum MappedFx {
    Native    { id: String, params: Vec<EParam> },
    Community { name: String, fallback: String, props: Vec<ZParam> },
}
#[derive(Clone)]
enum EParam { Scalar(u32, f32), Color(u32, [f32; 4]) }
#[derive(Clone)]
enum ZParam { Scalar(String, f32), Color(String, [f32; 4]) }

/// Read every `customProperty` of a community effect as a named scalar/color.
fn read_custom_props(e: &Value) -> Vec<ZParam> {
    let mut out = Vec::new();
    let Some(arr) = e.get("customProperties").and_then(|c| c.as_array()) else { return out };
    for cp in arr {
        let Some(name) = cp.get("properties").and_then(|p| p.get("name")).and_then(|n| n.as_str()) else { continue };
        if let Some(objs) = cp.get("objects").and_then(|o| o.as_array()) {
            if objs.len() >= 3 {
                let ch = |i: usize| objs.get(i).and_then(|c| c.get("keyframes")).and_then(|k| k.as_array())
                    .and_then(|k| k.first()).and_then(|kf| kf.get("value")).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                out.push(ZParam::Color(name.to_string(), [ch(0), ch(1), ch(2), 1.0]));
            }
        } else if let Some(v) = cp.get("keyframes").and_then(|k| k.as_array()).and_then(|k| k.first())
            .and_then(|kf| kf.get("value")).and_then(|v| v.as_f64())
        {
            out.push(ZParam::Scalar(name.to_string(), v as f32));
        }
    }
    out
}

/// Map a Panzoid built-in effect `type` string → our registry id. Colour overlays
/// return None here — they're baked into the text fill colour instead.
fn map_effect_type(t: &str) -> Option<&'static str> {
    Some(match t {
        "dropshadow"         => "drop_shadow",
        "echo"               => "echo",
        "glow"               => "glow",
        "boxblur"            => "box_blur",
        "brightnesscontrast" => "brightness_contrast",
        "edgedetection"      => "find_edges",
        "gradientoverlay"    => "multi_gradient",
        _ => return None, // coloroverlay → baked fill colour
    })
}

/// Map a `type:1` community-plugin effect by its display name → our registry id.
fn map_effect_name(name: &str) -> Option<&'static str> {
    let n = name.to_lowercase();
    Some(if n.contains("inner shadow")      { "inner_shadow" }
         else if n.contains("inner glow")   { "inner_glow" }
         else if n.contains("light sweep")  { "cc_light_sweep" }
         else if n.contains("stroke")       { "stroke" } // built-in fallback (zoidium port preferred)
         else if n.contains("drop shadow")  { "drop_shadow" }
         else if n.contains("glow")         { "glow" }
         else { return None }) // e.g. "AZ Light Burst", "Extrude" → no analogue
}

/// Recursively collect mappable effects from an `effects` array, descending into
/// `type:0` groups (nested `objects`). Keeps order and duplicates. For effects we
/// understand, also extracts param overrides.
fn collect_effects(list: &Value, out: &mut Vec<MappedFx>) {
    let Some(arr) = list.as_array() else { return };
    for e in arr {
        // Group node → recurse its children, don't map the container itself.
        if let Some(objs) = e.get("objects") {
            if objs.is_array() { collect_effects(objs, out); }
        }
        match e.get("type") {
            // Panzoid built-in effect (string type) → our native effect.
            Some(Value::String(ts)) => {
                if let Some(id) = map_effect_type(ts) {
                    out.push(MappedFx::Native { id: id.to_string(), params: effect_params(id, e) });
                }
            }
            // Community effect (numeric type) carrying a GLSL shader + customProperties
            // → forward by name (host picks a zoidium port, else the built-in fallback).
            _ => {
                if e.get("customProperties").is_some() {
                    if let Some(name) = e.get("properties").and_then(|p| p.get("name")).and_then(|n| n.as_str()) {
                        out.push(MappedFx::Community {
                            name:     name.to_string(),
                            fallback: map_effect_name(name).unwrap_or("").to_string(),
                            props:    read_custom_props(e),
                        });
                    }
                }
            }
        }
    }
}

/// Extract param overrides for the effects we know how to translate (others keep
/// our defaults). Indices match our effect param schemas.
fn effect_params(id: &str, e: &Value) -> Vec<EParam> {
    let props = e.get("properties");
    let mut out = Vec::new();
    match id {
        // drop_shadow: [0 color, 1 opacity, 2 distance, 3 angle, 4 softness]
        "drop_shadow" => {
            if let Some(c) = anim_rgb(prop(props, "color")) { out.push(EParam::Color(0, [c[0], c[1], c[2], 1.0])); }
            if let Some(v) = anim_f32(prop(props, "opacity"))  { out.push(EParam::Scalar(1, v)); }
            if let Some(v) = anim_f32(prop(props, "distance")) { out.push(EParam::Scalar(2, v)); }
            if let Some(v) = anim_f32(prop(props, "angle"))    { out.push(EParam::Scalar(3, v)); }
            if let Some(v) = anim_f32(prop(props, "blur"))     { out.push(EParam::Scalar(4, v)); }
        }
        // echo: [0 echo_time(s), 1 num, 2 start, 3 decay]
        "echo" => {
            if let Some(n) = pval(props, "echoes")  { out.push(EParam::Scalar(1, n.min(30.0))); }
            if let Some(o) = pval(props, "opacity") { out.push(EParam::Scalar(2, o)); }
            if let Some(d) = pval(props, "decay")   { out.push(EParam::Scalar(3, d)); }
            // Panzoid delay is in frames; our echo_time is seconds (negative = trailing).
            if let Some(dl) = pval(props, "delay")  { out.push(EParam::Scalar(0, -(dl / 30.0))); }
        }
        // box_blur: [0 radius]. Panzoid `delta` is a normalized amount → px (guess).
        "box_blur" => {
            if let Some(d) = pval(props, "delta") { out.push(EParam::Scalar(0, (d * 60.0).clamp(0.0, 400.0))); }
        }
        // find_edges: [1 blend-with-original]. Panzoid amount 1 = full edges → blend 0.
        "find_edges" => {
            if let Some(a) = pval(props, "amount") { out.push(EParam::Scalar(1, (1.0 - a).clamp(0.0, 1.0))); }
        }
        // multi_gradient: [0 opacity, 1 angle, 2 color1,3 pos1, 4 color2,5 pos2, …, 10 color5,11 pos5]
        "multi_gradient" => {
            if let Some(v) = anim_f32(prop(props, "opacity"))  { out.push(EParam::Scalar(0, v)); }
            // Panzoid rotation 0 ≈ our default 180 (vertical); add any rotation on top.
            let rot = anim_f32(prop(props, "rotation")).unwrap_or(0.0);
            out.push(EParam::Scalar(1, 180.0 + rot));
            if let Some(g) = prop(props, "gradient").and_then(|g| g.as_array()) {
                let n = g.len().min(5);
                let last = g.get(n.saturating_sub(1))
                    .and_then(|s| s.get("color")).and_then(|c| c.as_str())
                    .map(parse_rgba).unwrap_or([1.0; 4]);
                for i in 0..5 {
                    let (col, pos) = if i < n {
                        let s = &g[i];
                        let c = s.get("color").and_then(|c| c.as_str()).map(parse_rgba).unwrap_or([1.0; 4]);
                        let p = s.get("position").and_then(|p| p.as_f64()).unwrap_or(1.0) as f32;
                        (c, p)
                    } else {
                        (last, 1.0) // collapse unused stops onto the end
                    };
                    out.push(EParam::Color(2 + 2 * i as u32, col));
                    out.push(EParam::Scalar(3 + 2 * i as u32, pos));
                }
            }
        }
        _ => {}
    }
    out
}

fn prop<'a>(props: Option<&'a Value>, key: &str) -> Option<&'a Value> { props.and_then(|p| p.get(key)) }

/// Read a property value that may be animated (`{keyframes:[{value}]}`) or a plain
/// number — returns the first/raw scalar.
fn pval(props: Option<&Value>, key: &str) -> Option<f32> {
    let v = prop(props, key)?;
    anim_f32(Some(v)).or_else(|| v.as_f64().map(|x| x as f32))
}

/// First keyframe value of an animated scalar property.
fn anim_f32(p: Option<&Value>) -> Option<f32> {
    p.and_then(|p| p.get("keyframes")).and_then(|k| k.as_array()).and_then(|k| k.first())
        .and_then(|kf| kf.get("value")).and_then(|v| v.as_f64()).map(|v| v as f32)
}

/// First keyframe value of an animated RGB property (value is a `[r,g,b]` array 0..1).
fn anim_rgb(p: Option<&Value>) -> Option<[f32; 3]> {
    let arr = p.and_then(|p| p.get("keyframes")).and_then(|k| k.as_array()).and_then(|k| k.first())
        .and_then(|kf| kf.get("value")).and_then(|v| v.as_array())?;
    Some([arr.first()?.as_f64()? as f32, arr.get(1)?.as_f64()? as f32, arr.get(2)?.as_f64()? as f32])
}

fn run(gz: &[u8]) -> Result<String, String> {
    // 1. gunzip → tar bytes.
    let mut tar = Vec::new();
    flate2::read::GzDecoder::new(gz).read_to_end(&mut tar).map_err(|e| format!("gunzip: {e}"))?;

    // 2. untar → the `project` entry.
    let project = untar_find(&tar, "project").ok_or("archive has no 'project' entry")?;

    // 3. parse the scene JSON.
    let v: Value = serde_json::from_slice(&project).map_err(|e| format!("json parse: {e}"))?;

    // 4. comp first (Panzoid has no fixed resolution → 1920x1080; dur from sequence).
    let dur = v.get("sequence").and_then(|s| s.get("length")).and_then(|l| l.as_f64())
        .unwrap_or(260.0).max(1.0) as f32;
    octane::create_comp("Panzoid Import", 1920, 1080, 30.0, dur);
    octane::add_solid("BG", 1920.0, 1080.0, [0.03, 0.04, 0.06, 1.0], 960.0, 540.0);

    // 5. import embedded assets (fonts + audio), keyed by hash in `assets`.
    let (mut fonts, mut audios) = (0u32, 0u32);
    if let Some(assets) = v.get("assets").and_then(|a| a.as_object()) {
        for (hash, meta) in assets {
            let filename = meta.get("filename").and_then(|f| f.as_str()).unwrap_or("");
            let ext = filename.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
            let is_font  = matches!(ext.as_str(), "ttf" | "otf");
            let is_audio = matches!(ext.as_str(), "mp3" | "wav" | "ogg" | "m4a" | "aac");
            if !(is_font || is_audio) { continue; }
            if let Some(blob) = untar_find(&tar, hash) {
                if is_font { octane::import_font(hash, filename, &blob); fonts += 1; }
                else       { octane::import_audio(filename, &blob);      audios += 1; }
            }
        }
    }

    // 6. Walk the timeline: each video track's clips → a layer with its own in/out
    //    timing. "Scene" clips carry visual objects (text here); "Adjustment" clips
    //    become adjustment layers. Tracks added in order (track 0 = bottom).
    const K: f32 = 15.0; // world units → comp pixels (calibrated by eye)
    let (mut text_n, mut adj_n, mut skipped) = (0u32, 0u32, 0u32);

    if let Some(tracks) = v.get("sequence").and_then(|s| s.get("videoTracks")).and_then(|t| t.as_array()) {
        for track in tracks {
            let Some(clips) = track.get("clips").and_then(|c| c.as_array()) else { continue };
            for clip in clips {
                let name  = clip.get("properties").and_then(|p| p.get("name")).and_then(|n| n.as_str()).unwrap_or("");
                let start = clip.get("start").and_then(|s| s.as_f64()).unwrap_or(0.0) as f32;
                let len   = clip.get("length").and_then(|l| l.as_f64()).unwrap_or(dur as f64).max(1.0) as f32;
                let (in_f, out_f) = (start, start + len);
                let Some(obj) = clip.get("object") else { continue };

                if name.eq_ignore_ascii_case("Adjustment") {
                    // Adjustment layer carrying the clip's effect stack.
                    let mut fx = Vec::new();
                    if let Some(e) = obj.get("effects") { collect_effects(e, &mut fx); }
                    octane::add_adjustment("Adjustment");
                    octane::set_last_timing(in_f, out_f);
                    emit_effects(&fx);
                    adj_n += 1;
                } else {
                    // Scene clip: realise each text object it contains.
                    let mut texts = Vec::new();
                    collect_text(obj, [1.0; 4], None, &[], &[], &mut texts);
                    if texts.is_empty() { skipped += 1; continue; } // non-text visual — not handled yet
                    for t in &texts {
                        let px = 960.0 + t.x * K;
                        let py = 540.0 - t.y * K;
                        let size = (t.size * 9.0).clamp(28.0, 220.0);
                        octane::add_text_font(&t.text, px, py, size, &t.font, t.color);
                        octane::set_last_timing(in_f, out_f);
                        emit_effects(&t.effects);
                        if (t.scale[0] - 1.0).abs() > 1e-3 || (t.scale[1] - 1.0).abs() > 1e-3 || t.rot.abs() > 1e-3 {
                            octane::set_last_transform(t.scale[0], t.scale[1], t.rot);
                        }
                        // Opacity animation (fade). Keyframe frames are clip-local →
                        // offset by the clip start. Hold @ clip start = first value.
                        if !t.opacity_keys.is_empty() {
                            octane::set_last_opacity_key(in_f, t.opacity_keys[0].1);
                            for (f, val) in &t.opacity_keys { octane::set_last_opacity_key(in_f + *f, *val); }
                        }
                        text_n += 1;
                    }
                }
            }
        }
    }

    Ok(format!("imported {text_n} text, {adj_n} adjustment layer(s), {skipped} non-text clip(s) skipped; \
                {fonts} font(s), {audios} audio"))
}

/// Add each mapped effect to the layer just created, with its params.
fn emit_effects(fx: &[MappedFx]) {
    for f in fx {
        match f {
            MappedFx::Native { id, params } => {
                octane::add_effect(id);
                for p in params {
                    match p {
                        EParam::Scalar(i, v) => octane::set_effect_scalar(*i, *v),
                        EParam::Color(i, c)  => octane::set_effect_color(*i, *c),
                    }
                }
            }
            MappedFx::Community { name, fallback, props } => {
                octane::community_add_effect(name, fallback);
                for p in props {
                    match p {
                        ZParam::Scalar(n, v) => octane::community_set_scalar(n, *v),
                        ZParam::Color(n, c)  => octane::community_set_color(n, *c),
                    }
                }
            }
        }
    }
}

/// Minimal tar reader: return the data of the first entry named `want`.
fn untar_find(tar: &[u8], want: &str) -> Option<Vec<u8>> {
    let mut off = 0usize;
    while off + 512 <= tar.len() {
        let hdr = &tar[off..off + 512];
        let name_len = hdr[0..100].iter().position(|&b| b == 0).unwrap_or(100);
        let name = std::str::from_utf8(&hdr[0..name_len]).ok()?;
        if name.is_empty() { break; } // two zero blocks terminate the archive
        // size: octal ASCII in bytes [124, 136).
        let size_field = std::str::from_utf8(&hdr[124..136]).unwrap_or("");
        let size = usize::from_str_radix(size_field.trim_matches(|c| c == '\0' || c == ' '), 8).unwrap_or(0);
        let data_start = off + 512;
        if name == want {
            return tar.get(data_start..data_start + size).map(|s| s.to_vec());
        }
        off = data_start + size.div_ceil(512) * 512;
    }
    None
}

/// Recursively collect text objects, propagating color down the tree. A node's
/// `material` sets the base fill; an ancestor/self `effects` overlay
/// (color/gradient) overrides it (that's what's actually seen). A text object is a
/// node whose `properties` holds the `text`.
fn collect_text(v: &Value, mat: [f32; 4], overlay: Option<[f32; 4]>, fx: &[MappedFx],
                okeys: &[(f32, f32)], out: &mut Vec<TextObj>) {
    match v {
        Value::Object(m) => {
            let mut mat = mat;
            let mut overlay = overlay;
            if let Some(c) = m.get("material").and_then(material_color) { mat = c; }
            if let Some(c) = m.get("effects").and_then(overlay_color)   { overlay = Some(c); }

            // Inherit the nearest animated opacity down the path (clip fade-in).
            // An object's animated channels live under its `properties`.
            let mut okeys = okeys.to_vec();
            let these = read_scalar_keys(m.get("properties").and_then(|p| p.get("opacity")));
            if !these.is_empty() { okeys = these; }

            // Accumulate mappable stylize effects on the path (groups recursed,
            // order + duplicates preserved).
            let mut fx = fx.to_vec();
            if let Some(list) = m.get("effects") { collect_effects(list, &mut fx); }

            if let Some(props) = m.get("properties").and_then(|p| p.as_object()) {
                if let Some(Value::String(s)) = props.get("text") {
                    if !s.trim().is_empty() {
                        let x = read_chan(props.get("position"), 0).unwrap_or(0.0);
                        let y = read_chan(props.get("position"), 1).unwrap_or(0.0);
                        let scale = [read_chan(props.get("scale"), 0).unwrap_or(1.0),
                                     read_chan(props.get("scale"), 1).unwrap_or(1.0)];
                        let rot = read_chan(props.get("rotation"), 2).unwrap_or(0.0); // euler Z (deg)
                        let size = props.get("size")
                            .and_then(|s| s.get(0).and_then(|n| n.as_f64()).or_else(|| s.as_f64()))
                            .unwrap_or(3.0) as f32;
                        let font = props.get("font").and_then(|f| f.as_str()).unwrap_or("").to_string();
                        out.push(TextObj { text: s.clone(), x, y, size, font,
                            color: overlay.unwrap_or(mat), effects: fx.clone(), scale, rot,
                            opacity_keys: okeys.clone() });
                    }
                }
            }
            for vv in m.values() { collect_text(vv, mat, overlay, &fx, &okeys, out); }
        }
        Value::Array(a) => for vv in a { collect_text(vv, mat, overlay, fx, okeys, out); },
        _ => {}
    }
}

/// Keyframes `(frame, value)` of an animated scalar channel (empty if ≤1 keyframe).
fn read_scalar_keys(ch: Option<&Value>) -> Vec<(f32, f32)> {
    let kfs = match ch.and_then(|c| c.get("keyframes")).and_then(|k| k.as_array()) {
        Some(k) if k.len() >= 2 => k,
        _ => return Vec::new(),
    };
    kfs.iter().filter_map(|kf| Some((
        kf.get("frame")?.as_f64()? as f32,
        kf.get("value")?.as_f64()? as f32,
    ))).collect()
}

/// Read a Panzoid `{objects:[R,G,B]}` colour (channels 0..1) → rgb.
fn read_rgb01(objects: Option<&Value>) -> Option<[f32; 3]> {
    let arr = objects?.as_array()?;
    let ch = |i: usize| -> f32 {
        arr.get(i).and_then(|c| c.get("keyframes")).and_then(|k| k.as_array())
            .and_then(|k| k.first()).and_then(|kf| kf.get("value")).and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32
    };
    Some([ch(0), ch(1), ch(2)])
}

/// Base colour from a `singlecolor` material.
fn material_color(mat: &Value) -> Option<[f32; 4]> {
    if mat.get("type").and_then(|t| t.as_str()) == Some("singlecolor") {
        let rgb = read_rgb01(mat.get("properties").and_then(|p| p.get("color")).and_then(|c| c.get("objects")))?;
        return Some([rgb[0], rgb[1], rgb[2], 1.0]);
    }
    None
}

/// Fill colour from a top-level `coloroverlay` (gradient overlays are imported as
/// a gradient_overlay effect instead, not baked into the fill).
fn overlay_color(effects: &Value) -> Option<[f32; 4]> {
    for e in effects.as_array()? {
        if e.get("type").and_then(|t| t.as_str()) == Some("coloroverlay") {
            if let Some(rgb) = read_rgb01(e.get("properties").and_then(|p| p.get("color")).and_then(|c| c.get("objects"))) {
                return Some([rgb[0], rgb[1], rgb[2], 1.0]);
            }
        }
    }
    None
}

/// Parse `"rgba(r, g, b, a)"` (rgb 0..255) → rgba 0..1.
fn parse_rgba(s: &str) -> [f32; 4] {
    let inner = s.trim().trim_start_matches("rgba").trim_start_matches("rgb")
        .trim_matches(|c| c == '(' || c == ')' || c == ' ');
    let mut it = inner.split(',').map(|p| p.trim().parse::<f32>().unwrap_or(0.0));
    let r = it.next().unwrap_or(0.0);
    let g = it.next().unwrap_or(0.0);
    let b = it.next().unwrap_or(0.0);
    let a = it.next().unwrap_or(1.0);
    [r / 255.0, g / 255.0, b / 255.0, a]
}

/// Base value of channel `i` of a Panzoid vec3 property `{objects:[X,Y,Z]}`, each
/// `{keyframes:[{value, ...}]}`. Used for position / scale / rotation.
fn read_chan(p: Option<&Value>, i: usize) -> Option<f32> {
    p.and_then(|p| p.get("objects")).and_then(|o| o.as_array()).and_then(|o| o.get(i))
        .and_then(|c| c.get("keyframes")).and_then(|k| k.as_array()).and_then(|k| k.first())
        .and_then(|kf| kf.get("value")).and_then(|v| v.as_f64()).map(|v| v as f32)
}

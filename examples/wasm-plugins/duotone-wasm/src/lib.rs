//! Sample WASM plugin (Track W). Registers a Duotone effect via the WGSL hand-off
//! plus a trivial command. Build:
//!
//!   cargo build --target wasm32-unknown-unknown --release
//!
//! then copy `target/wasm32-unknown-unknown/release/duotone_wasm.wasm` into the
//! plugins dir (`%APPDATA%\OctaneEditor\plugins\`).

use octane_plugin_sdk as octane;

// Same manifest/WGSL contract as a file plugin — just handed over in memory. A
// distinct id so it coexists with the file-plugin `duotone`.
const MANIFEST: &str = r#"(
    id:       "duotone_wasm",
    name:     "Duotone (WASM)",
    category: "color",
    icon:     "levels",
    params: [
        (id: "shadow",    label: "Shadow Color",    kind: Color(default: (0.04, 0.09, 0.26, 1.0))),
        (id: "highlight", label: "Highlight Color", kind: Color(default: (1.00, 0.74, 0.24, 1.0))),
        (id: "amount",    label: "Amount",          kind: Percent(default: 1.0, min: 0.0, max: 1.0)),
    ],
    passes: [ (src: 0, dst: 1) ],
)"#;

const SHADER: &str = r#"
struct FxUniforms { data: array<vec4<f32>,6>, texel: vec2<f32>, pass_idx: u32, time: f32 };
@group(0) @binding(0) var<uniform> u: FxUniforms;
@group(0) @binding(1) var t_src: texture_2d<f32>;
@group(0) @binding(2) var s_sampler: sampler;
struct VertexOutput { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    out.clip = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv   = vec2<f32>(x, 1.0 - y);
    return out;
}
const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = textureSample(t_src, s_sampler, in.uv);
    let a = c.a;
    let rgb = select(c.rgb / max(a, 1e-5), vec3<f32>(0.0), a <= 0.0001);
    let l      = dot(rgb, LUMA);
    let duo    = mix(u.data[0].rgb, u.data[1].rgb, l);
    let outRgb = mix(rgb, duo, u.data[2].x);
    return vec4<f32>(outRgb * a, a);
}
"#;

/// Called once by the host at load.
#[no_mangle]
pub extern "C" fn octane_register() {
    octane::log("duotone-wasm: registering");
    octane::set_plugin_name("Duotone (WASM)");   // shown in the Plugins menu
    octane::register_effect(MANIFEST, SHADER);
    octane::register_command("Build Solid Grid", "build_grid");
    octane::register_command("Stagger Selected (diagonal)", "stagger_selected");
}

/// Automation command demonstrating the read+mutate scene-API: offset each
/// selected layer diagonally by an increasing step (a quick "cascade" layout).
#[no_mangle]
pub extern "C" fn stagger_selected() {
    let n = octane::selected_count();
    if n == 0 { octane::log("stagger: nothing selected"); return; }
    octane::log(&format!("staggering {n} selected layer(s)"));
    let step = 48.0f32;
    for i in 0..n {
        let id = octane::selected_id(i);
        let (x, y) = (octane::layer_x(id), octane::layer_y(id));
        octane::set_position(id, x + i as f32 * step, y + i as f32 * step);
    }
}

/// Automation command: generate a 4×3 grid of solid layers tinted by position.
/// Runs sandboxed; the `add_solid` calls are applied by the host as one undo step.
#[no_mangle]
pub extern "C" fn build_grid() {
    octane::log("building a 4x3 solid grid");
    let (cols, rows) = (4u32, 3u32);
    let (cell, gap)  = (180.0f32, 40.0f32);
    let step = cell + gap;
    // Centre the grid roughly around a 1920x1080 comp.
    let ox = 960.0 - (cols as f32 - 1.0) * step * 0.5;
    let oy = 540.0 - (rows as f32 - 1.0) * step * 0.5;
    for y in 0..rows {
        for x in 0..cols {
            let r = x as f32 / (cols - 1) as f32;
            let g = y as f32 / (rows - 1) as f32;
            octane::add_solid(
                &format!("Grid {}x{}", x + 1, y + 1),
                cell, cell, [r, g, 0.65, 1.0],
                ox + x as f32 * step, oy + y as f32 * step,
            );
        }
    }
}

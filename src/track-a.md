# Track A — File plugins (effects)

A file plugin is a folder with a manifest and a shader. Drop it into a plugins
dir and it becomes a new effect — no recompile, no code.

## Folder layout

```
duotone/
├── manifest.ron   # identity + parameter schema + pass list
└── effect.wgsl    # the full-screen fragment shader
```

## `manifest.ron`

```ron
(
    id:       "duotone",        // STABLE unique id (also the shader id). Saved in
                                // projects — never change it once shipped.
    name:     "Duotone",        // shown in menus
    category: "color",          // blur|color|stylize|generate|distort|transition|motion|time
    icon:     "levels",         // blur|levels|hue|tint|shadow|glow

    params: [
        (id: "shadow",    label: "Shadow Color",    kind: Color(default: (0.10, 0.02, 0.22, 1.0))),
        (id: "highlight", label: "Highlight Color", kind: Color(default: (1.00, 0.86, 0.30, 1.0))),
        (id: "amount",    label: "Amount",          kind: Percent(default: 1.0, min: 0.0, max: 1.0)),
    ],

    passes: [ (src: 0, dst: 1) ],   // one full-screen pass: read buf 0 → write buf 1

    // Optional:
    //   shader:  "effect.wgsl",    // shader filename (default "effect.wgsl")
    //   buffers: 2,                // scratch buffers (default 2, minimum 2)
    //   zoidium: "Stroke",         // tag this as a Panzoid port (see the Zoidium chapter)
)
```

### Parameter kinds

Max **6** params — see [Limits](#hard-limits-of-track-a).

```ron
kind: Scalar(default: 10.0, min: 0.0, max: 100.0, speed: 1.0, suffix: " px")
kind: Percent(default: 1.0, min: 0.0, max: 1.0)   // shown as 0–100%
kind: Angle(default: 0.0)                          // degrees, dial UI, 0°=up/CW
kind: Color(default: (1.0, 0.5, 0.0, 1.0))         // rgba 0..1
kind: Bool(default: false)
kind: Choice(options: ["Low", "High"], default: 0)
```

`Scalar`, `Percent`, and `Angle` are **keyframable**. All params get a stopwatch
and the full timeline/graph treatment for free.

## `effect.wgsl` — the shader contract

**Copy the binding header and `vs_main` verbatim.** Only write `fs_main`.

```wgsl
// Duotone — map luminance onto a shadow→highlight ramp.
//   data[0] = shadow.rgba   data[1] = highlight.rgba   data[2] = (amount, _, _, _)
struct FxUniforms {
    data:     array<vec4<f32>, 6>,   // your params, packed one vec4 per entry
    texel:    vec2<f32>,             // (1/width, 1/height) — for neighbour taps
    pass_idx: u32,                   // 0-based index of the current pass
    time:     f32,                   // comp time in seconds (animate with this)
};

@group(0) @binding(0) var<uniform> u:         FxUniforms;
@group(0) @binding(1) var          t_src:     texture_2d<f32>;
@group(0) @binding(2) var          s_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0)       uv:   vec2<f32>,
}

// Standard full-screen triangle. Copy verbatim.
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
    // Input is PREMULTIPLIED: divide by alpha to get straight colour.
    let rgb = select(c.rgb / max(a, 1e-5), vec3<f32>(0.0), a <= 0.0001);

    let l      = dot(rgb, LUMA);
    let duo    = mix(u.data[0].rgb, u.data[1].rgb, l);  // shadow → highlight by luma
    let outRgb = mix(rgb, duo, u.data[2].x);            // amount

    return vec4<f32>(outRgb * a, a);                    // re-PREMULTIPLY on output
}
```

### How params reach the shader

Params pack **one `vec4` per entry, in manifest order** — **param N is `u.data[N]`**:

| kind | read as |
|------|---------|
| Scalar / Percent / Angle | `u.data[N].x` |
| Color | `u.data[N]` (rgba) |
| Bool | `u.data[N].x` (0.0 or 1.0) |
| Choice | `u.data[N].x` (selected index as float) |

### Premultiplied alpha

Every image in the pipeline is **premultiplied**. Divide by alpha to get straight
colour, do your work, multiply alpha back on output (as above). Skipping this
produces dark fringes on edges.

## Multi-pass & buffers

`passes` is a list of `(src, dst)` scratch-buffer index pairs, run in order. All
passes share the **one** shader and the **same** packed params; branch on
`u.pass_idx`. Input arrives in **buffer 0**; the last `dst` is the result.

```ron
// Example: a separable blur — horizontal (0→1) then vertical (1→0).
passes:  [ (src: 0, dst: 1), (src: 1, dst: 0) ],
buffers: 2,
```

Bump `buffers` if you need more than 2 scratch targets.

## Hard limits of Track A

- **6 params max** (the uniform is 6×`vec4` = 24 floats).
- Full-screen passes only: **colour, distortion, generators, patterns**.
- **No** multi-input (can't sample another layer), **no** geometry, **no**
  stateful simulation. Those remain built-in (or need a native track later).

If you need logic — generating layers, reading the scene, importing a file — move
to [Track W](./track-w.md).

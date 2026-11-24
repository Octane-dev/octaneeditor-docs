# Porting community effects (Zoidium)

The Panzoid `.pz` importer is itself a bundled WASM plugin. When it meets a
Panzoid community effect it doesn't translate the shader inline — it asks the
host for a **port**:

```rust
// inside the importer
octane::community_add_effect("Stroke (Extended)", "stroke"); // name, built-in fallback
octane::community_set_scalar("width", 2.5);
octane::community_set_color("color", [1.0, 1.0, 1.0, 1.0]);
```

A **port** is an ordinary effect (Track A or W) whose manifest carries a
`zoidium:` tag naming the Panzoid effect it reproduces:

```ron
(
    id:      "zoidium_stroke",
    name:    "Zoidium Stroke",
    category: "stylize",
    zoidium: "Stroke (Extended)",   // ← the host matches community_add_effect by this, case-insensitive
    params: [
        (id: "color", label: "Color", kind: Color(default: (1.0,1.0,1.0,1.0))),
        (id: "width", label: "Width", kind: Scalar(default: 0.42, min: 0.0, max: 16.0, speed: 0.05)),
        // ...
    ],
    passes: [ (src: 0, dst: 1) ],
)
```

## Resolution order

When the importer calls `community_add_effect(name, fallback)`:

1. A registered effect whose `zoidium:` tag equals `name` (case-insensitive) → use it.
2. Else the built-in `fallback` id, if any.
3. Else skip.

`community_set_scalar` / `community_set_color` then set params **by `id`**
(case-insensitive), so a port just needs matching param ids.

> **This is how you add new Panzoid effect support:** write a WGSL port, tag it
> `zoidium: "<Panzoid name>"`, ship it. The importer picks it up automatically —
> no change to the importer needed.

## Translating the shader (GLSL → WGSL)

Panzoid shaders are **WebGL GLSL ES** fragment shaders. Porting one means
rewriting it against the WGSL `fs_main` contract from the
[Track A chapter](./track-a.md#effectwgsl--the-shader-contract):

| WebGL GLSL ES | WGSL (Octane contract) |
|---|---|
| `texture2D(tex, uv)` / `texture(tex, uv)` | `textureSample(t_src, s_sampler, in.uv)` |
| `gl_FragColor = c;` | `return c;` |
| `varying vec2 vUv;` | the `in.uv` input |
| `uniform vec2 iResolution;` | derive from `u.texel` (`1.0 / u.texel`) |
| `uniform float iTime;` | `u.time` |
| the effect's own uniforms | `u.data[N]` (one `vec4` per param) |

Remember the images are **premultiplied** — divide by alpha, work, re-multiply.

## Authoring form

**Edit ▸ Manage Plugins…** has an in-app **Zoidium port form**: paste the Panzoid
effect name + your WGSL, define the params, and it writes the `manifest.ron` +
`effect.wgsl` into your user plugins folder for you. The id is derived from the
name (e.g. `"Stroke (Extended)"` → `zoidium_stroke_extended`).

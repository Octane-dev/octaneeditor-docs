# SDK reference

Everything a Track W plugin can call (`octane_plugin_sdk`). Positions are comp
pixels (top-left origin); colours are rgba `0..1`.

```rust
// ── registration (call from octane_register) ─────────────────────────────
log(msg)
register_effect(manifest_ron, wgsl)
register_command(name, export_fn)
register_importer(name, &extensions, export_fn)

// ── read scene (valid during a command/importer) ─────────────────────────
comp_width() / comp_height() / comp_fps() / comp_duration() -> f32
layer_count() -> u32            layer_id(index) -> LayerId
layer_selected(id) -> bool      layer_x(id) / layer_y(id) -> f32
selected_count() -> u32         selected_id(index) -> LayerId

// ── create layers ────────────────────────────────────────────────────────
add_solid(name, w, h, color, x, y)   add_null(name, x, y)
add_adjustment(name)                 add_text(text, x, y, size)
add_text_font(text, x, y, size, font_id, color)

// ── mutate by id ─────────────────────────────────────────────────────────
set_position(id, x, y)          set_scale(id, sx, sy)
set_rotation(id, deg)           set_opacity(id, pct)
set_position_key(id, frame, x, y)

// ── "most recently added" layer ──────────────────────────────────────────
set_last_timing(in, out)        set_last_opacity_key(frame, value)
set_last_transform(sx, sy, rot_deg)

// ── effects on the most recently added layer ─────────────────────────────
add_effect(type_id)             set_effect_scalar(index, value)
set_effect_color(index, color)
community_add_effect(name, fallback)
community_set_scalar(prop, value)    community_set_color(prop, color)

// ── assets & comps ───────────────────────────────────────────────────────
import_font(font_id, name, &bytes)   import_audio(name, &bytes)
create_comp(name, w, h, fps, duration_frames)

// ── importer input ───────────────────────────────────────────────────────
input_bytes() -> Vec<u8>
```

## manifest.ron reference (both tracks)

| Field | Required | Meaning |
|---|---|---|
| `id` | ✅ | stable unique id; also the shader id. Saved in projects — never change. |
| `name` | ✅ | display name in menus |
| `category` | | `blur` \| `color` \| `stylize` \| `generate` \| `distort` \| `transition` \| `motion` \| `time` |
| `icon` | | `blur` \| `levels` \| `hue` \| `tint` \| `shadow` \| `glow` |
| `params` | | up to **6** param entries (see below) |
| `passes` | | list of `(src, dst)` buffer-index pairs |
| `shader` | | shader filename (default `effect.wgsl`) — Track A only |
| `buffers` | | scratch buffers (default 2, min 2) |
| `zoidium` | | Panzoid effect name this effect ports |

### Param kinds

```ron
Scalar(default, min, max, speed, suffix)   // keyframable; read u.data[N].x
Percent(default, min, max)                 // keyframable; 0..1 shown as 0–100%
Angle(default)                             // keyframable; degrees, dial UI
Color(default: (r, g, b, a))               // rgba 0..1; read u.data[N]
Bool(default)                              // read u.data[N].x (0 or 1)
Choice(options: [...], default)            // read u.data[N].x (index)
```

## WGSL binding contract

```wgsl
struct FxUniforms {
    data:     array<vec4<f32>, 6>,   // param N → data[N]
    texel:    vec2<f32>,             // (1/w, 1/h)
    pass_idx: u32,                   // current pass index
    time:     f32,                   // comp seconds
};
@group(0) @binding(0) var<uniform> u:         FxUniforms;
@group(0) @binding(1) var          t_src:     texture_2d<f32>;
@group(0) @binding(2) var          s_sampler: sampler;
```

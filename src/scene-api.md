# The scene API

All of these are free functions on `octane_plugin_sdk` (Track W). **Reads** are
valid only during a command/importer call (they sample the live scene at the
playhead). **Mutations** are queued and applied as one undo step when your
function returns. Positions are **composition pixels, top-left origin**; colours
are rgba `0..1`.

## Read

```rust
comp_width()  -> f32      comp_height()   -> f32
comp_fps()    -> f32      comp_duration() -> f32   // frames

layer_count()          -> u32            // bottom-to-top
layer_id(index: u32)   -> LayerId        // handle at z-order index (0 = bottom)
layer_selected(id)     -> bool
layer_x(id) -> f32       layer_y(id) -> f32        // at the playhead

selected_count()       -> u32
selected_id(index: u32)-> LayerId
```

## Create layers

```rust
add_solid(name, w, h, color:[f32;4], x, y)
add_null(name, x, y)
add_adjustment(name)                               // affects everything beneath
add_text(text, x, y, size)
add_text_font(text, x, y, size, font_id, color)    // font_id from import_font (""=default)
```

## Mutate layers by id

```rust
set_position(id, x, y)
set_scale(id, sx, sy)             // 1.0 = 100%
set_rotation(id, deg)
set_opacity(id, pct)              // 0..100
set_position_key(id, frame, x, y) // position keyframe
```

## Edits to the "most recently added" layer

Many ops target the last layer you added — handy while building a scene:

```rust
set_last_timing(in_frame, out_frame)
set_last_opacity_key(frame, value)   // value 0..1; call repeatedly for a fade
set_last_transform(scale_x, scale_y, rotation_deg)
```

## Effects on the most recently added layer

```rust
add_effect(type_id)                   // e.g. "drop_shadow", "glow", "echo"
set_effect_scalar(index, value)       // param by index
set_effect_color(index, [r,g,b,a])

// Community / ported effects, addressed by NAME (see the Zoidium chapter):
community_add_effect(name, fallback)  // uses a registered port if present, else `fallback` id
community_set_scalar(prop, value)     // param by name
community_set_color(prop, [r,g,b,a])
```

## Assets & new comps

```rust
import_font(font_id, name, &bytes)    // TTF/OTF → tag it `font_id` for add_text_font
import_audio(name, &bytes)            // encoded audio (e.g. mp3) → asset + audio layer
create_comp(name, width, height, fps, duration_frames)  // and make it active
```

## Importer input

```rust
input_bytes() -> Vec<u8>              // raw bytes of the file being imported
```

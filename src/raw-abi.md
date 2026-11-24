# Writing plugins in another language (raw ABI)

Track W plugins talk to the host through a **raw WebAssembly ABI**, not a
Rust-specific API. The [`octane-plugin-sdk`](./track-w.md) crate is only a thin
convenience wrapper over the calls documented here — so **any language that
compiles to `wasm32` can write a plugin** (C, C++, Zig, AssemblyScript, TinyGo,
hand-written WAT, …) by importing these host functions and exporting the entry
points directly.

> Prefer Rust? Use the SDK and skip this page. This is for everyone else.

## Hard requirements for the module

1. **Target `wasm32-unknown-unknown`** (no WASI). The host instantiates the module
   with **only** the `octane` import module available.
2. **Import nothing but `octane.*`.** Any other import (WASI calls, an
   `env.abort`, a GC runtime hook, …) makes instantiation **fail** — the host has
   no definition for it. Strip or stub them.
   - *AssemblyScript:* compile with `--runtime stub` and remove the default
     `env.*` imports (`--use abort=…`), or it won't instantiate.
   - *C/Zig (freestanding wasm):* fine out of the box.
3. **Export your linear memory as `memory`.** The host reads/writes your memory to
   pass strings and file bytes. (Most toolchains export `memory` by default.)
4. **Export `octane_register`** (signature `() -> ()`), plus one `() -> ()` export
   per command / importer you register.

## Strings & memory

There are no string types on the boundary. A string is **UTF-8 bytes in your
linear memory, passed as two `i32` args: `ptr, len`**. The host reads them
synchronously during the call, so a pointer that's valid at call time is enough
(no lifetime tricks needed).

For **importers**, data flows the other way: ask the host how many bytes the file
is (`octane_input_len`), allocate a buffer in your memory, then call
`octane_read_input(ptr, len)` — the host copies the file bytes into your buffer.

## Value types

| Wasm type | Used for |
|-----------|----------|
| `i32` | a pointer, a byte length, or a small integer / bool (`0`/`1`) |
| `i64` | a `LayerId` — an opaque layer handle (`0` = none / not found) |
| `f32` | any float — comp pixels (top-left origin), colors (`0.0..1.0` rgba), scale (`1.0` = 100%), angles (degrees), frames, seconds |

## Lifecycle

1. Host instantiates your module (providing the `octane` imports).
2. Host calls **`octane_register()`** once. Inside, call `octane_set_plugin_name`
   and the `octane_register_*` functions to declare what you provide. These just
   record metadata host-side.
3. When the user runs a **command**, the host calls your exported command
   function (no args). During that call you may **read** the scene
   (`octane_comp_*`, `octane_layer_*`, `octane_selected_*`) and **queue
   mutations** (`octane_scene_*`, `octane_*effect*`, …). All queued mutations are
   applied as **one undo step** after your function returns.
4. For an **importer**, the host calls your exported importer function; read the
   file via `octane_input_len`/`octane_read_input`, then build the scene
   (typically `octane_create_comp` then `octane_scene_add_*`).

Scene **reads are only valid inside** a command/importer call.

## Host imports (module `"octane"`)

All in import module `octane`. `ptr`/`len`/`idx` are `i32`.

### Registration & logging (call from `octane_register`)

| Function | Signature | Meaning |
|---|---|---|
| `octane_log` | `(ptr, len)` | print a UTF-8 message to the host console |
| `octane_set_plugin_name` | `(ptr, len)` | display name in the Plugins menu (else the file stem is prettified) |
| `octane_register_effect` | `(m_ptr, m_len, w_ptr, w_len)` | register an effect: RON manifest + WGSL source (see [Track A](./track-a.md)) |
| `octane_register_command` | `(n_ptr, n_len, f_ptr, f_len)` | name + the export-fn name the host calls to run it |
| `octane_register_importer` | `(n_ptr, n_len, e_ptr, e_len, f_ptr, f_len)` | name + comma-separated extensions + export-fn name |

### Read the scene (valid during a command/importer)

| Function | Signature | Returns |
|---|---|---|
| `octane_comp_width` / `octane_comp_height` | `() -> f32` | comp size (px) |
| `octane_comp_fps` | `() -> f32` | frame rate |
| `octane_comp_duration` | `() -> f32` | duration (frames) |
| `octane_layer_count` | `() -> i32` | layer count (bottom-to-top) |
| `octane_layer_id` | `(index: i32) -> i64` | handle at z-order `index` (`0` if OOB) |
| `octane_layer_selected` | `(id: i64) -> i32` | `1` if selected |
| `octane_layer_x` / `octane_layer_y` | `(id: i64) -> f32` | position at the playhead |
| `octane_selected_count` | `() -> i32` | number selected |
| `octane_selected_id` | `(index: i32) -> i64` | handle of the `index`-th selected |

### Create layers (queued)

| Function | Signature |
|---|---|
| `octane_scene_add_solid` | `(n_ptr, n_len, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32, x: f32, y: f32)` |
| `octane_scene_add_null` | `(n_ptr, n_len, x: f32, y: f32)` |
| `octane_scene_add_adjustment` | `(n_ptr, n_len)` |
| `octane_scene_add_text` | `(t_ptr, t_len, x: f32, y: f32, size: f32)` |
| `octane_scene_add_text_font` | `(t_ptr, t_len, f_ptr, f_len, x: f32, y: f32, size: f32, r: f32, g: f32, b: f32, a: f32)` — `f_*` = a font id from `octane_import_asset` |

### Mutate a layer by id (queued)

| Function | Signature |
|---|---|
| `octane_scene_set_position` | `(id: i64, x: f32, y: f32)` |
| `octane_scene_set_scale` | `(id: i64, sx: f32, sy: f32)` — `1.0` = 100% |
| `octane_scene_set_rotation` | `(id: i64, deg: f32)` |
| `octane_scene_set_opacity` | `(id: i64, pct: f32)` — `0..100` |
| `octane_scene_set_position_key` | `(id: i64, frame: f32, x: f32, y: f32)` |

### The "most recently added" layer (queued)

| Function | Signature |
|---|---|
| `octane_set_last_timing` | `(in_frame: f32, out_frame: f32)` |
| `octane_set_last_opacity_key` | `(frame: f32, value: f32)` — `value` is `0..1` |
| `octane_set_last_transform` | `(sx: f32, sy: f32, rot_deg: f32)` |

### Effects on the most recently added layer (queued)

| Function | Signature |
|---|---|
| `octane_add_effect` | `(id_ptr, id_len)` — registry type id, e.g. `"drop_shadow"` |
| `octane_set_effect_scalar` | `(index: i32, value: f32)` — param by index |
| `octane_set_effect_color` | `(index: i32, r: f32, g: f32, b: f32, a: f32)` |
| `octane_community_add_effect` | `(n_ptr, n_len, f_ptr, f_len)` — name + fallback id (see [Zoidium](./zoidium.md)) |
| `octane_community_set_scalar` | `(p_ptr, p_len, value: f32)` — param by name |
| `octane_community_set_color` | `(p_ptr, p_len, r: f32, g: f32, b: f32, a: f32)` |

### Assets, comps & importer input

| Function | Signature | Meaning |
|---|---|---|
| `octane_import_asset` | `(id_ptr, id_len, kind: i32, n_ptr, n_len, d_ptr, d_len)` | import bytes; `kind` `0` = font, `1` = audio. `id_*` tags a font for `add_text_font` (pass `0,0` for audio) |
| `octane_create_comp` | `(n_ptr, n_len, w: i32, h: i32, fps: f32, dur: f32)` | create + activate a comp (for importers) |
| `octane_input_len` | `() -> i32` | byte length of the file being imported |
| `octane_read_input` | `(ptr, len)` | host copies up to `len` input bytes into your memory at `ptr` |

## Guest exports you provide

| Export | Signature | When the host calls it |
|---|---|---|
| `memory` | (linear memory) | always — the host reads/writes it |
| `octane_register` | `() -> ()` | once, at load |
| *your command fns* | `() -> ()` | when the user runs that command |
| *your importer fns* | `() -> ()` | when the user opens a matching file |

## Minimal example (WAT sketch)

A no-op plugin that just logs on load and registers one command — the shape any
language reduces to:

```wat
(module
  (import "octane" "octane_log"              (func $log  (param i32 i32)))
  (import "octane" "octane_register_command" (func $cmd  (param i32 i32 i32 i32)))
  (memory (export "memory") 1)
  ;; "hi" at 0, "Do Thing" at 8, "do_thing" at 24
  (data (i32.const 0)  "hi")
  (data (i32.const 8)  "Do Thing")
  (data (i32.const 24) "do_thing")
  (func (export "octane_register")
    (call $log (i32.const 0) (i32.const 2))
    (call $cmd (i32.const 8) (i32.const 8) (i32.const 24) (i32.const 8)))
  (func (export "do_thing")))   ;; body would read/mutate the scene
```

In a real language you'd write normal code; the compiler emits exactly this
import/export/memory shape. Match the signatures above and you're a first-class
plugin — same pipeline, menu, and sandbox as a Rust one.

> **Roadmap:** a future move to the WASM **Component Model** (WIT interfaces +
> `wit-bindgen`) would auto-generate idiomatic bindings for many languages, making
> this hand-rolling unnecessary. Same wasmtime engine underneath — an interface
> upgrade, not a rewrite.

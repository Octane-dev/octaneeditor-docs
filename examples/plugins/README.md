# OctaneEditor plugins (Track A — data + WGSL effects)

A **plugin** is a folder containing a `manifest.ron` and a WGSL shader. Drop it
into a plugins dir and it becomes a new effect — no recompile, no code:

- **User (writable):** `%APPDATA%\OctaneEditor\plugins\<name>\`
- **Bundled (shipped, read-only):** `<install dir>\plugins\<name>\`

Both are scanned at startup; a user plugin wins over a bundled one with the same
`id`. A plugin effect gets a generated FX-panel UI, keyframable parameters, and
save/load — identical to a built-in effect.

## Installing & managing (Edit ▸ Manage Plugins…)

The **Plugin Manager** lists every installed plugin and lets you:

- **Install…** a `.octane-plugin` package (see below),
- **enable / disable** a plugin (unchecking skips it at next launch),
- **Remove** a user plugin (bundled ones are read-only),
- open the user plugins folder.

Because effects + GPU pipelines are built once at startup, changes apply on the
**next launch** (the window shows a "restart to apply" banner).

### Packaging a `.octane-plugin`

A `.octane-plugin` is just a **zip** of the plugin folder (so the archive
contains `duotone/manifest.ron`, `duotone/effect.wgsl`, …). The installer reads
the `id` from the manifest and installs into `plugins\<id>\`. A root-level zip
(manifest at the archive root) also works — the common folder prefix is stripped.

## Folder layout

```
duotone/
├── manifest.ron   # identity + parameter schema + pass list
└── effect.wgsl    # the full-screen fragment shader
```

## manifest.ron

See [`duotone/manifest.ron`](duotone/manifest.ron) for an annotated example.
Fields: `id` (stable, unique — also the shader id), `name`, `category`
(`blur|color|stylize|generate|distort|transition|motion|time`), `icon`
(`blur|levels|hue|tint|shadow|glow`), `params`, `passes`, and optional `shader`
(filename, default `effect.wgsl`) / `buffers` (default 2).

### Parameter kinds

```ron
kind: Scalar(default: 10.0, min: 0.0, max: 100.0, speed: 1.0, suffix: " px")
kind: Percent(default: 1.0, min: 0.0, max: 1.0)
kind: Angle(default: 0.0)
kind: Color(default: (1.0, 0.5, 0.0, 1.0))      // rgba 0..1
kind: Bool(default: false)
kind: Choice(options: ["Low", "High"], default: 0)
```

## The shader contract

Copy the binding header and `vs_main` from [`duotone/effect.wgsl`](duotone/effect.wgsl)
verbatim. Parameters pack **one `vec4` per entry, in manifest order**, into
`u.data[]` — **param N is `u.data[N]`** (so max 6 params):

| kind | read as |
|------|---------|
| Scalar / Percent / Angle | `u.data[N].x` |
| Color | `u.data[N]` (rgba) |
| Bool | `u.data[N].x` (0 or 1) |
| Choice | `u.data[N].x` (selected index) |

Images are **premultiplied**: divide by alpha to get straight colour, do your
work, then multiply alpha back in on output (see the example).

Multiple `passes` share the one shader and the same packed params — chain them
through distinct scratch buffers via `(src, dst)` indices (bump `buffers` if you
need more than 2).

## Limits (Track A)

Single/multi full-screen passes only: colour, distortion, generators, patterns.
No multi-input, geometry, or stateful simulation — those stay built-in (or wait
for the native/WASM plugin track).

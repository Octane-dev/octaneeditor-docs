# OctaneEditor Plugin Handbook

This is the complete guide to extending OctaneEditor with plugins. You can add
new **effects**, **automation commands**, and **file importers** — without
recompiling the app, and (for WASM) without any ability to crash or escape the
host sandbox.

- **No Rust required** for simple effects — a text manifest + a WGSL shader is a
  full plugin (Track A).
- **Rust compiled to WASM** when you need logic: commands that read/mutate the
  scene, or importers for new file formats (Track W).

> **Golden rule:** effects, pipelines, and menus are built **once at startup**.
> Installing, enabling, or disabling a plugin takes effect on the **next launch**
> (the Plugin Manager shows a "restart to apply" banner).

## The two tracks at a glance

| | **Track A — File plugin** | **Track W — WASM plugin** |
|---|---|---|
| What it is | A folder: `manifest.ron` + `effect.wgsl` | A `.wasm` module (Rust `cdylib`) |
| Can add | Effects (GPU shader) | Effects **+** commands **+** importers |
| Language | RON + WGSL | Rust (any lang that targets `wasm32`) |
| Reads/mutates the scene? | No | Yes (via the host scene API) |
| Build step | None | `cargo build --target wasm32-unknown-unknown` |
| Sandbox | n/a (shader only) | Yes — cannot crash host or touch disk/net |

Both tracks feed the **same** effect registry and GPU pipeline, so a plugin
effect is indistinguishable from a built-in: generated FX-panel UI, keyframable
params, save/load.

**A WASM effect uses the exact same manifest + WGSL contract as a file plugin** —
it just hands the two strings to the host in memory instead of via files. Learn
Track A first; Track W builds on it.

## Where to start

- Want to add a **visual effect**? → [Track A](./track-a.md).
- Want a plugin that **generates or rearranges a scene**, or **imports a file
  format**? → [Track W](./track-w.md) and [The scene API](./scene-api.md).
- Porting a **Panzoid / community effect**? → [Zoidium](./zoidium.md).

Full runnable examples ship in the [`examples/`](https://github.com/Octane-dev/octaneeditor-docs/tree/main/examples)
folder of this repo.

# Track W — WASM plugins

A WASM plugin is a Rust `cdylib` compiled to `wasm32-unknown-unknown` that
depends on the **`octane-plugin-sdk`** crate and exports `octane_register()`.
Unlike a native DLL, it **cannot crash the host or escape the sandbox** — it only
reaches the host functions the SDK exposes.

## Project setup

`Cargo.toml`:

```toml
[package]
name    = "mything"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
octane-plugin-sdk = "0.1"

[profile.release]
opt-level = "s"
lto       = true

# Detached from the main workspace — built explicitly for wasm32.
[workspace]
```

Build & install:

```sh
rustup target add wasm32-unknown-unknown          # once
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/mything.wasm \
   "$APPDATA/OctaneEditor/plugins/"               # Windows: %APPDATA%\OctaneEditor\plugins\
```

## The entry point

The host calls `octane_register()` **once** at load. Inside it, register
everything your plugin contributes. Nothing else is called until the user runs a
command or opens a matching file.

```rust
use octane_plugin_sdk as octane;

#[no_mangle]
pub extern "C" fn octane_register() {
    octane::log("mything loading");
    octane::register_effect(MANIFEST, SHADER);                     // effect
    octane::register_command("Build Solid Grid", "build_grid");    // command
    octane::register_importer("My Format (.myf)", &["myf"], "import_myf"); // importer
}
```

> The SDK compiles on non-wasm targets too (all host calls become no-ops), so
> `cargo check` on your dev machine works for IDE/autocomplete.

## Registering an effect (same contract as Track A)

```rust
const MANIFEST: &str = r#"(
    id:   "duotone_wasm",
    name: "Duotone (WASM)",
    category: "color",
    icon: "levels",
    params: [
        (id: "shadow",    label: "Shadow Color",    kind: Color(default: (0.04, 0.09, 0.26, 1.0))),
        (id: "highlight", label: "Highlight Color", kind: Color(default: (1.00, 0.74, 0.24, 1.0))),
        (id: "amount",    label: "Amount",          kind: Percent(default: 1.0, min: 0.0, max: 1.0)),
    ],
    passes: [ (src: 0, dst: 1) ],
)"#;

const SHADER: &str = r#" /* same WGSL contract as the Track A chapter */ "#;

// in octane_register():
octane::register_effect(MANIFEST, SHADER);
```

The WASM instance is only needed at load time — once the host has the manifest +
WGSL it builds the identical GPU effect and the module can go to sleep.

## Commands (automation)

A command is a menu action. Register a display name + the name of a `#[no_mangle]`
export; the host calls that export when the user runs it. Inside, **read** the
scene and **queue** edits — the host applies all queued edits as **one undo step**
after the function returns.

```rust
/// Offset each selected layer diagonally by an increasing step.
#[no_mangle]
pub extern "C" fn stagger_selected() {
    let n = octane::selected_count();
    if n == 0 { octane::log("nothing selected"); return; }
    let step = 48.0;
    for i in 0..n {
        let id = octane::selected_id(i);
        let (x, y) = (octane::layer_x(id), octane::layer_y(id));
        octane::set_position(id, x + i as f32 * step, y + i as f32 * step);
    }
}

/// Generate a 4×3 grid of tinted solids (applied as one undo step).
#[no_mangle]
pub extern "C" fn build_grid() {
    let (cols, rows) = (4u32, 3u32);
    let step = 220.0;
    let ox = octane::comp_width()  * 0.5 - (cols as f32 - 1.0) * step * 0.5;
    let oy = octane::comp_height() * 0.5 - (rows as f32 - 1.0) * step * 0.5;
    for y in 0..rows {
        for x in 0..cols {
            let (r, g) = (x as f32 / 3.0, y as f32 / 2.0);
            octane::add_solid(&format!("Grid {}x{}", x+1, y+1),
                180.0, 180.0, [r, g, 0.65, 1.0],
                ox + x as f32 * step, oy + y as f32 * step);
        }
    }
}
```

See [The scene API](./scene-api.md) for everything a command can read and change.

## Importers (new file formats)

Register for one or more extensions (lower-case, no dot). When the user
opens/drops a matching file the host calls your export; read the bytes with
`input_bytes()`, typically `create_comp()` then `add_*`.

```rust
#[no_mangle]
pub extern "C" fn import_myf() {
    let bytes = octane::input_bytes();          // the file's raw bytes
    octane::create_comp("My Import", 1920, 1080, 30.0, 150.0);
    // ...parse `bytes`, then build the scene with add_text / add_solid / ...
    octane::add_text("Hello from .myf", 960.0, 540.0, 96.0);
}
```

The bundled Panzoid `.pz` importer is a full real-world example — see the
[`examples/`](https://github.com/Octane-dev/octaneeditor-docs/tree/main/examples)
folder.

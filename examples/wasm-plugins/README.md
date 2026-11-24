# OctaneEditor WASM plugins (Track W)

Sandboxed, compiled plugins loaded via **wasmtime**. A plugin is a `.wasm` file
dropped into a plugins dir (`%APPDATA%\OctaneEditor\plugins\`, or a sub-folder).
Unlike a native DLL, a WASM plugin **cannot crash the host or escape the
sandbox** — it only reaches the host functions we expose.

## How it works

A plugin exports `octane_register()`, which the host calls once at load. Inside,
the plugin calls host imports (via the `octane-plugin-sdk` crate):

- `register_effect(manifest_ron, wgsl)` — contributes an effect through the
  **WGSL hand-off**. Same manifest + shader contract as a file plugin (see
  `../plugins/README.md`); the host builds the identical `PluginEffect`, so WASM
  effects run on the GPU exactly like built-ins. The wasm instance is only needed
  at load time.
- `register_command(name, export_fn)` — registers an automation command whose
  body is a `#[no_mangle]` export. *(v1 collects these; a scene host-API + a
  command menu to invoke them is the next step.)*
- `log(msg)` — prints through the host console.

## Building a plugin

See [`duotone-wasm/`](duotone-wasm/) for a complete example.

```sh
cd duotone-wasm
cargo build --target wasm32-unknown-unknown --release
# → target/wasm32-unknown-unknown/release/duotone_wasm.wasm
cp target/wasm32-unknown-unknown/release/duotone_wasm.wasm \
   "$APPDATA/OctaneEditor/plugins/"
```

Prereq: `rustup target add wasm32-unknown-unknown`. The plugin is a `cdylib`
depending on `octane-plugin-sdk`; it has its own `[workspace]` so it builds
independently of the main app.

## Current scope & roadmap

- **Done:** load/sandbox, entry point, host imports, effect hand-off (verified).
- **Next:** invoke commands from a menu + a scene host-API (add layer, set
  keyframe, …) so plugins can automate the app.
- **Later:** upgrade the ABI to the WASM **Component Model** (typed WIT
  interfaces via `cargo-component`) for a polished public SDK — same wasmtime
  engine, so it's an interface upgrade, not a rewrite.

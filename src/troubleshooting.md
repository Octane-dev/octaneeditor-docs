# Troubleshooting

| Symptom | Cause / fix |
|---|---|
| Plugin doesn't appear | Wrong dir, or a parse error — check the console (`[plugins]` / `[wasm:...]` lines). Remember: **restart** to apply. |
| Effect added but no visual change | Shader reads the wrong `u.data[N]` index, or forgot to re-premultiply. |
| Dark fringe on edges | Not dividing by / multiplying back alpha (premultiplied). |
| More than 6 params needed | Not possible on Track A — the uniform is fixed at 6×`vec4`. Split into multiple effects. |
| WASM command does nothing | The `export_fn` name in `register_command` must exactly match a `#[no_mangle] pub extern "C" fn`. |
| Importer never fires | Extension mismatch — register it lower-case, no dot (`&["pz"]`). |
| `community_set_scalar` has no effect | Param name must match a param `id` on the resolved effect (case-insensitive). |
| Changes don't take until I edit again | Known: some add paths don't flush the RAM-preview cache; any further edit forces a re-render. |

## Reading the log

The host prints plugin activity to its console:

- `[plugins] …` — Track A discovery / manifest parse errors.
- `[wasm:<name>] …` — anything your plugin sends via `octane::log(...)`, plus
  load/instantiation errors.

A broken plugin is **logged and skipped** — it never takes down the host.

# Packaging & distribution

## The `.octane-plugin` format

A `.octane-plugin` is **just a ZIP** of the plugin folder.

- **Track A:** zip the folder so the archive contains `duotone/manifest.ron`,
  `duotone/effect.wgsl`, … (a manifest at the archive root also works — the
  common folder prefix is stripped).
- **Track W:** include the `.wasm` (plus any bundled companion files, e.g. a
  ported-effect folder — the `.pz` importer ships this way).

On install, the host reads `id` from the manifest and unpacks into
`plugins\<id>\`. Path-traversal (`..`) entries are rejected.

## Installing

Three ways:

1. **Edit ▸ Manage Plugins… ▸ Install…** — pick a `.octane-plugin` file.
2. Drop the plugin folder (Track A) or `.wasm` (Track W) straight into
   `%APPDATA%\OctaneEditor\plugins\`.
3. For bundled plugins, use the one-click **install bundled** button in the
   Plugin Manager.

Either way: **restart to apply** (effects + pipelines build at startup).

## Enabling / disabling

The Plugin Manager's checkbox writes a disabled list; disabled plugins are
skipped at the next launch. User plugins can be **Removed** entirely; bundled
ones are read-only.

## Sharing

- Zip your folder → rename to `.octane-plugin` → share the file (Discord, a
  release asset, wherever).
- Users double-click **Install…** in the Plugin Manager, or unzip into their
  plugins dir by hand.

# Where plugins live

| Location | Path | Notes |
|---|---|---|
| **User** (writable) | `%APPDATA%\OctaneEditor\plugins\` | where you install your own |
| **Bundled** (read-only) | `<install dir>\plugins\` | ships with the app |

Both are scanned at startup. On an `id` collision, the **user** plugin wins.

- **Track A:** one sub-folder per plugin — `plugins\duotone\manifest.ron` + `effect.wgsl`.
- **Track W:** a `.wasm` file directly in the dir, or one level deep — `plugins\mything.wasm`.

Manage everything from **Edit ▸ Manage Plugins…**:

- **Install…** a `.octane-plugin` package (see [Packaging](./packaging.md)),
- **enable / disable** a plugin (unchecking skips it at next launch),
- **Remove** a user plugin (bundled ones are read-only),
- open the user plugins folder,
- one-click **install bundled** plugins,
- author a **Zoidium port** with the in-app form (see [Zoidium](./zoidium.md)).

> Because effects + GPU pipelines are built once at startup, changes apply on the
> **next launch** — the window shows a "restart to apply" banner.

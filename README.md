# OctaneEditor Plugin Handbook

Public developer documentation for writing **OctaneEditor** plugins — effects,
automation commands, and file importers.

📖 **Read it online:** https://Octane-dev.github.io/octaneeditor-docs/

This repo is an [mdBook](https://rust-lang.github.io/mdBook/). The main app lives
in a separate (private) repository; only the public docs + example plugins are
here.

## Build locally

```sh
cargo install mdbook        # once
mdbook serve --open         # live-reload preview at http://localhost:3000
mdbook build                # output in ./book
```

## Publishing

Pushing to `main` triggers `.github/workflows/deploy.yml`, which builds the book
and deploys it to GitHub Pages. Enable it once under **Settings ▸ Pages ▸ Source:
GitHub Actions**.

## Layout

```
src/            handbook chapters (Markdown) + SUMMARY.md (table of contents)
examples/       copy-and-go example plugins
book.toml       mdBook config
```

## Contributing

Edit the Markdown in `src/`, run `mdbook serve` to preview, open a PR.

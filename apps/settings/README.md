# kanami settings window

Tauri 2 + Svelte 5. It starts on demand and exits when the window is closed. Remapping is done by the background service (`kanami`); the settings window reads state, sends commands and subscribes to key events over local IPC. For the design, see [docs/design/gui.md](../../docs/design/gui.md).

## Building

Requires Node.js (20 or newer) and Rust.

```bash
cd apps/settings
npm ci
npm run build                    # frontend output goes to dist/ and is bundled into the program
cargo build -p kanami-settings   # run from the repository root; produces target/debug/kanami-settings
```

The settings window is not a default workspace member, so `cargo build` / `cargo test` at the repository root do not build it (`dist/` must exist first).

The background service looks for `kanami-settings` in its own directory, which is how "Open Settings..." in the menu bar finds it. During development, put both programs in the same directory.

To package `Kanami.app`:

```bash
npx tauri build
```

## Development

```bash
npx tauri dev    # starts the Vite dev server; frontend changes apply immediately
```

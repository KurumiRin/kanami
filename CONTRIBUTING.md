# Contributing

Thank you for contributing to kanami.

## Development setup

- Rust 1.90 or newer (install with `rustup`)
- macOS: full development needs macOS. `kanami-core` and `kanami-config` build and test on any platform.

## Before submitting

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same checks.

## Project structure

| Crate | Responsibility |
|---|---|
| `kanami-core` | Platform-independent event model and rule engine; uses no system APIs |
| `kanami-config` | Parses the TOML configuration into rules |
| `kanami-script` | Luau script plugin runtime |
| `kanami-i18n` | Message catalog and system language detection |
| `kanami-ipc` | Local IPC between the background service and the settings window |
| `kanami-macos` | macOS backend, based on `CGEventTap` |
| `kanami-windows` | Windows backend, based on a low-level keyboard hook |
| `kanami` | Command-line entry point |
| `apps/settings` | Settings window (Tauri 2 + Svelte 5), see [apps/settings/README.md](apps/settings/README.md) |

For more design notes, see [docs/architecture.md](docs/architecture.md).

## Conventions

- **Hot path**: do not allocate, lock, do I/O or log in event callbacks or `Engine::process`.
- **Platform isolation**: system APIs appear only in platform backend crates. `kanami-core` stays pure logic.
- **Tests**: new rule behavior needs unit tests in `kanami-core`. A new key name needs the key code mapping of every platform, and a test checks for omissions.
- **JSON Schema**: after changing key names, the modifier syntax or the configuration structure, regenerate the schema with `cargo run -- schema > schema/kanami.schema.json`. A test checks that it is up to date.
- **Example configuration**: the `[[test]]` entries in `config.example.toml` run during `cargo test`. Update them when rule semantics change.
- **Dependencies**: explain the reason for each new dependency. Binary size is one of the project's core metrics.
- **Comments, docs and UI text**: comments and docs are in English. User-visible text goes through `tr!` (Rust) or `t()` (settings window), with translations in `crates/kanami-i18n/src/zh_cn.rs` and `apps/settings/src/lib/locales/zh-CN.js`; `cargo test -p kanami-i18n` and `npm run check:i18n` check coverage. Add a language by adding one catalog file on each side.

## Filing issues

- Bugs: include the macOS version, chip architecture, kanami version, the configuration file and the relevant output of `kanami monitor`.
- Features: describe the use case first, then the expected behavior.

## License

Unless you state otherwise, contributions you submit are released under the dual MIT or Apache-2.0 license.

## What changed

<!-- A short description, and why the change is needed -->

## Checklist

- [ ] `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` and `cargo test` all pass
- [ ] The hot path (event callbacks, `Engine::process`) gains no allocation, lock or I/O
- [ ] User-visible changes are reflected in `CHANGELOG.md` and the README

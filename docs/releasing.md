# Releasing (maintainers)

## Publishing a version

1. Update `version` under `[workspace.package]` in `Cargo.toml`, then run `cargo build` to refresh `Cargo.lock`.
2. Move the "Unreleased" entries in `CHANGELOG.md` under a heading for the new version.
3. Check locally: `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test`.
4. Commit and tag: `git tag v0.1.0 && git push origin v0.1.0`. A tag containing `-` (such as `v0.2.0-beta.1`) is published as a pre-release.
5. `.github/workflows/release.yml` builds the macOS (Apple Silicon and Intel) and Windows versions, attaches SHA-256 checksum files and creates the GitHub Release.
6. Update the Homebrew tap: change `url` in `packaging/homebrew/kanami.rb` to the new tag, and set `sha256` to the output of

   ```bash
   curl -sL https://github.com/KurumiRin/kanami/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256
   ```

   then commit it to the tap repository.

## About signing

The project has no paid Apple Developer account, so:

- **No notarization.** Gatekeeper blocks a downloaded binary, and users need to remove the quarantine attribute: `xattr -dr com.apple.quarantine kanami`. Installing from source with Homebrew is recommended, because locally compiled programs are not affected.
- **Ad-hoc signing only** (`codesign --sign -`). macOS privacy permissions (Accessibility, Input Monitoring) identify a program by its signature, and an ad-hoc signature differs on every build, so **users must grant the permissions again after every upgrade**. This is common among open-source tools without a developer account (skhd and kanata behave the same way) and should be mentioned in the release notes.
- If the project gets a developer account later, the release workflow should switch to Developer ID signing and notarization, which lets permissions persist across versions.

The Windows build is not code-signed, so SmartScreen may show an "Unknown publisher" warning.

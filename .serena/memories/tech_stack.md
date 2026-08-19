# Toolchain
- Rust 2021 workspace managed by Cargo; versions are centralized in the root `Cargo.toml` workspace package/dependencies.
- `kompanion_extractor` depends on `zip` for archive handling, `epub-stream` for EPUB parsing, and local core/sys crates; it is built as a `cdylib` with colocated unit tests.
- `kompanion_launcher/build.rs` is a host build script that reads workspace/package files and writes generated `kpm` artifacts; it must remain independent of target-only Kindle dependencies.
- CI builds x86 tests plus `armv7-unknown-linux-gnueabihf` and `armv7-unknown-linux-gnueabi` release targets.
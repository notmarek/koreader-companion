# Completion Checks
- Run `cargo fmt --check` for Rust edits.
- Run `cargo test -p kompanion_extractor`; run `cargo test --workspace` when practical.
- Run `cargo build -p kompanion_launcher` and inspect generated `kpm/install.sql` and `kpm/uninstall.sql` when changing build-time package generation.
- Review `git diff` and ensure only intended source/template/generated changes are included; do not revert unrelated worktree changes.
# Commands
- Run extractor tests with `cargo test -p kompanion_extractor`.
- Run workspace tests with `cargo test --workspace` when practical.
- Trigger package SQL/manifest generation with `cargo build -p kompanion_launcher` or a workspace build.
- Verify formatting with `cargo fmt --check`.
- Inspect changes with `git status --short`, `git diff`, and `git log --oneline -10`.
- The environment may not have the `rtk` command despite its shell guidance; use the underlying Cargo/Git command if `rtk` is unavailable.
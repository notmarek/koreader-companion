# Project Map
- Rust 2021 Cargo workspace: `kompanion_core`, `kompanion_sys`, `kompanion_extractor`, `kompanion_launcher`.
- `kompanion_extractor` is the Kindle scanner-loaded `cdylib`; `src/extractor.rs` handles scanner events and delegates supported files through `src/indexer/mod.rs`.
- Indexers implement `kompanion_core::indexer::FileIndexer`; the registry is the source of runtime extractor availability.
- `kompanion_launcher/build.rs` generates package metadata and SQL artifacts under `kpm/` during launcher builds; package scripts copy those artifacts into the kpkg.
- Installer SQL must use Kindle appreg's misspelled `extenstions` table exactly.
- Read `mem:tech_stack` for toolchain/dependencies, `mem:conventions` for Rust patterns, `mem:suggested_commands` for commands, and `mem:task_completion` for completion checks.
# Code Conventions
- Rust modules use compact structs implementing `FileIndexer`; indexer methods return `Result<_, String>` and format I/O/parser failures into strings.
- Unit tests are colocated in `#[cfg(test)] mod tests` at the bottom of implementation modules.
- Existing archive tests create temporary files, write ZIP fixtures with `zip::ZipWriter`, and assert trait behavior and filesystem outputs.
- Keep extractor registration deterministic and single-sourced where build-generated packaging metadata depends on it.
- Preserve existing Kindle SQL names and handler IDs; `extenstions` is intentional schema spelling.
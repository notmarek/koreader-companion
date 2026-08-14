# kindlepw2 Build Notes

## What this is

KOmpanion — a Kindle-side KPM package (NOT the vanadium23/kompanion server).
Source: https://github.com/notmarek/koreader-companion  
Our fork target: add kindlepw2 (armel/softfloat) support alongside existing kindlehf.

## What it does on the Kindle

- `libkompanion_extractor.so` — CCAT plugin: parses epub files so the Kindle library
  shows covers and metadata for epubs
- `kompanion_launcher` — default handler for `MT:application/epub+zip`: tapping an epub
  in the Kindle library opens KOReader instead of the built-in reader

Registered via `install.sql` into `/var/local/appreg.db` (interfaces: extractor + application).

## Build system

The project is **pure Rust** (cargo workspace, 4 crates). The CI uses meson but
there is NO `meson.build` in the repo — CI currently fails on all platform jobs.
The correct build path is `pack.sh` which uses cargo directly with koxtoolchain.

## What needs to change for kindlepw2

All the kindlepw2 plumbing is already in the repo but **commented out**:

### `.cargo/config.toml`
Currently only has `armv7-unknown-linux-gnueabihf` target. Need to add:
```toml
[target.armv7-unknown-linux-gnueabi]
linker = "arm-kindlepw2-linux-gnueabi-gcc"
```

### `Cargo.toml` (workspace root)
```toml
# Change:
supported_platforms = ["kindlehf"] #, "kindlepw2"]
# To:
supported_platforms = ["kindlehf", "kindlepw2"]
```

### `pack.sh`
Uncomment the kindlepw2 copy block:
```sh
mkdir -p build/kindlepw2/{bin,lib}
cp target/armv7-unknown-linux-gnueabi/release/kompanion_launcher build/kindlepw2/bin/
cp target/armv7-unknown-linux-gnueabi/release/libkompanion_extractor.so build/kindlepw2/lib/
```

### `kpm/install.sh`
Uncomment arch detection:
```sh
ARCH="kindlepw2"
[ -f "/usr/lib/ld-linux-armhf.so" ] && ARCH="kindlehf"
```
And change all `kindlehf` references to use `$ARCH`.

### `kpm/manifest.json`
Add `"kindlepw2"` to `supported_platforms`.

## Build process

Requires koxtoolchain (Linux binaries — needs Docker on macOS):

```bash
# In Docker (ubuntu:latest)
wget -q https://github.com/KindleModding/koxtoolchain/releases/latest/download/kindlepw2.tar.gz \
  -O - | tar -xzf - -C ~
wget -q https://github.com/KindleModding/koxtoolchain/releases/latest/download/kindlehf.tar.gz \
  -O - | tar -xzf - -C ~
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
rustup target add armv7-unknown-linux-gnueabi armv7-unknown-linux-gnueabihf
source ~/koxtoolchain/refs/x-compile.sh kindlepw2 env
cargo build --release --features real-lipc,real-scanner --target armv7-unknown-linux-gnueabi
```

## Binary dependencies at runtime

- `libscanner.so.1` — Kindle internal (present on kindlepw2)
- `liblipc.so.1` — Kindle internal (present on kindlepw2)
- `libgcc_s.so.1`, `libpthread.so.0`, `libc.so.6` — standard Kindle libs
- All epub parsing is statically compiled Rust (no libzip/libxml2 needed)

## Next steps

1. Make the code changes above
2. Build in Docker
3. Test on Kindle (SSH deploy)
4. Open PR to notmarek/koreader-companion
5. Add to our kindlepw2-kpm repo once binaries are confirmed working

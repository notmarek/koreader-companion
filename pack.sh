#!/bin/sh
set -e

# Build both architectures. Requires koxtoolchain bin dirs in PATH:
#   ~/x-tools/arm-kindlehf-linux-gnueabihf/bin
#   ~/x-tools/arm-kindlepw2-linux-gnueabi/bin

VERSION=$(jq -r '.version | join(".")' kpm/manifest.json)

# Kindle's libcjson.so lacks convenience add functions (pre-1.6.0); compile static fixup
arm-kindlehf-linux-gnueabihf-gcc -c -o /tmp/cjson_addboolfixup_hf.o stubs/cjson_addboolfixup.c
arm-kindlepw2-linux-gnueabi-gcc -c -o /tmp/cjson_addboolfixup_pw2.o stubs/cjson_addboolfixup.c
# kindlepw2 sysroot glibc 2.12 lacks getauxval (added in 2.16)
arm-kindlepw2-linux-gnueabi-gcc -c -o /tmp/getauxval_stub.o stubs/getauxval_stub.c

FEATURES="kompanion_sys/real-lipc,kompanion_sys/real-scanner"

# The cjson stub must only be linked into the extractor SO. Linking it into the
# launcher too makes the linker pull the sysroot libcjson.so and fail with
# "DSO missing from command line" (binutils >= 2.45).
RUSTFLAGS="-C link-arg=/tmp/cjson_addboolfixup_hf.o" \
  cargo build --release --target armv7-unknown-linux-gnueabihf \
    --features "$FEATURES" -p kompanion_extractor
cargo build --release --target armv7-unknown-linux-gnueabihf \
  --features "$FEATURES" -p kompanion_launcher

# kindlepw2: same split; the getauxval stub is needed by both binaries.
RUSTFLAGS="-C link-arg=/tmp/getauxval_stub.o -C link-arg=/tmp/cjson_addboolfixup_pw2.o" \
  cargo build --release --target armv7-unknown-linux-gnueabi \
    --features "$FEATURES" -p kompanion_extractor
RUSTFLAGS="-C link-arg=/tmp/getauxval_stub.o" \
  cargo build --release --target armv7-unknown-linux-gnueabi \
    --features "$FEATURES" -p kompanion_launcher

rm -rf build dist
mkdir dist
mkdir -p build/kindlehf/bin build/kindlehf/lib
mkdir -p build/kindlepw2/bin build/kindlepw2/lib
cp kpm/install.sh kpm/install.sql kpm/manifest.json kpm/uninstall.sh kpm/uninstall.sql build/
cp target/armv7-unknown-linux-gnueabihf/release/kompanion_launcher build/kindlehf/bin/
cp target/armv7-unknown-linux-gnueabihf/release/libkompanion_extractor.so build/kindlehf/lib/
cp target/armv7-unknown-linux-gnueabi/release/kompanion_launcher build/kindlepw2/bin/
cp target/armv7-unknown-linux-gnueabi/release/libkompanion_extractor.so build/kindlepw2/lib/
(cd build && python3 -c "
import tarfile, os, sys
out = sys.argv[1]
with tarfile.open(out, 'w:gz', compresslevel=5) as tar:
    for name in sorted(os.listdir('.')):
        if not name.startswith('.'):
            tar.add(name)
" "../dist/kompanion-${VERSION}.kpkg")

rm -rf build

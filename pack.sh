source ~/koxtoolchain/refs/x-compile.sh kindlehf env

cargo build-all

VERSION=$(jq -r '.version | join(".")' kpm/manifest.json)

rm -rf build dist
mkdir dist
mkdir -p build/kindlehf/{bin,lib}
cp kpm/* build/
cp target/armv7-unknown-linux-gnueabihf/release/kompanion_launcher build/kindlehf/bin/
cp target/armv7-unknown-linux-gnueabihf/release/libkompanion_extractor.so build/kindlehf/lib/
# mkdir -p build/kindlepw2/{bin,lib}
# cp target/armv7-unknown-linux-gnueabi/release/kompanion_launcher build/kindlepw2/bin/
# cp target/armv7-unknown-linux-gnueabi/release/libkompanion_extractor.so build/kindlepw2/lib/
(cd build && tar -czf ../dist/kompanion-${VERSION}.kpkg *)

rm -rf build

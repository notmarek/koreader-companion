source ~/koxtoolchain/refs/x-compile.sh kindlehf env
cargo build --release --target armv7-unknown-linux-gnueabihf --features=real-lipc,real-scanner
VERSION=$(jq -r '.version | join(".")' ./kpm/manifest.json)
mkdir -p build/{bin,lib}
mkdir -p dist
cp kpm/* build/
cp target/armv7-unknown-linux-gnueabihf/release/kompanion_launcher build/bin/
cp target/armv7-unknown-linux-gnueabihf/release/libkompanion_extractor.so build/lib/
(cd build && tar -czf ../kompanion.tar.gz *)
mv kompanion.tar.gz dist/kompanion-$VERSION.kpkg
rm -rf build

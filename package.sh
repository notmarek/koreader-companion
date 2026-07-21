source ~/koxtoolchain/refs/x-compile.sh kindlehf env
cargo build --release --target armv7-unknown-linux-gnueabihf --features=real-lipc,real-scanner
VERSION=$(jq -r '.version | join(".")' ./kpm/manifest.json)
mkdir -p build/{bin,lib}
mkdir -p dist
cp kpm/* build/
cp target/armv7-unknown-linux-gnueabihf/release/koreader_companion_launcher build/bin/
cp target/armv7-unknown-linux-gnueabihf/release/libkoreader_companion_extractor.so build/lib/
tar -czf koreader-companion.tar.gz -C build/ .
mv koreader-companion.tar.gz dist/koreader-companion-$VERSION.kpm
rm -rf build

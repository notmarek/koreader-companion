ARCH="kindlepw2"
[ -f "/usr/lib/ld-linux-armhf.so" ] && ARCH="kindlehf"

# Determine extractor destination
if [ -d "/usr/lib/ccat" ]; then
    EXTRACTOR_LIB_DIR="/usr/lib/ccat"
else
    EXTRACTOR_LIB_DIR="/var/local/kompanion/lib"
fi

# Install launcher
mkdir -p /var/local/kompanion/bin
cp -r ./$ARCH/bin/* /var/local/kompanion/bin/

# Install extractor
mkdir -p "$EXTRACTOR_LIB_DIR"
cp ./$ARCH/lib/libkompanion_extractor.so "$EXTRACTOR_LIB_DIR/"

# Register in appreg.db with correct paths
sed "s|@EXTRACTOR_LIB@|${EXTRACTOR_LIB_DIR}/libkompanion_extractor.so|" ./install.sql | sqlite3 /var/local/appreg.db

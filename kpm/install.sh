#ARCH="kindlepw2"
#[ -f "/usr/lib/ld-linux-armhf.so" ] && ARCH="kindlehf"

mkdir -p /var/local/kompanion
mv ./kindlehf/lib /var/local/kompanion
mv ./kindlehf/bin /var/local/kompanion
cat ./install.sql | sqlite3 /var/local/appreg.db

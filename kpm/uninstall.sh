#!/bin/sh
# Uninstall script for the KOmpanion

echo "Deleting extractor"

mntroot rw
rm -f "/usr/lib/ccat/libkompanion_extractor.so"
mntroot ro

rm -f "/var/local/kompanion/lib/libkompanion_extractor.so"


echo "Deleting launcher"
rm -rf "/var/local/kompanion"

if [ ! "$1" = "upgrade" ]; then
    echo "Removing registration from appreg.db"
    sqlite3 /var/local/appreg.db < ./uninstall.sql
fi

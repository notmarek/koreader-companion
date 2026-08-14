#!/bin/sh
# Uninstall script for the KOmpanion

echo "Deleting extractor"
rm -f "/usr/lib/ccat/libkompanion_extractor.so"
rm -f "/var/local/kompanion/lib/libkompanion_extractor.so"

echo "Deleting launcher"
rm -rf "/var/local/kompanion"

if [ ! "$1" = "upgrade" ]; then
    echo "Removing registration from appreg.db"
    sqlite3 /var/local/appreg.db < ./uninstall.sql
fi

#!/bin/sh
# Uninstall script for the Koreader Companion

echo "Deleting extractor"
rm -rf "/var/local/kmc/lib/libkoreader_companion_extractor.sh"

echo "Deleting launcher"
rm -rf "/var/local/kmc/bin/koreader_companion_launcher"

if [ ! "$1" = "upgrade" ]; then
    echo "Removing registration from appreg.db"
cat ./uninstall.sql | sqlite3 /var/local/appreg.db
fi

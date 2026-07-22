#!/bin/sh
# Uninstall script for the Koreader Companion

echo "Deleting extractor"
echo "Deleting launcher"
rm -rf "/var/local/kompanion"

if [ ! "$1" = "upgrade" ]; then
    echo "Removing registration from appreg.db"
cat ./uninstall.sql | sqlite3 /var/local/appreg.db
fi

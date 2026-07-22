mkdir -p /var/local/kompanion
mv ./lib /var/local/kompanion
mv ./bin /var/local/kompanion
cat ./install.sql | sqlite3 /var/local/appreg.db

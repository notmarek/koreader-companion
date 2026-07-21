mv ./lib /var/local/kmc 
mv ./bin /var/local/kmc
cat ./install.sql | sqlite3 /var/local/appreg.db

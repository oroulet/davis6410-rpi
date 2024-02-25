#!/bin/bash
set -e
set -v
DB="./db.sqlite"
if test -f "$DB";
then
rm $DB;
fi

sqlite3 $DB "CREATE TABLE IF NOT EXISTS wind (ts    REAL PRIMARY KEY, vel  REAL, direction  INTEGER)"


#!/bin/bash
# Levanta BD de prueba + mock de ML + servidor y corre la suite.
# Config por entorno: PGBASE (default postgres://postgres@localhost)
set -e
DIR="$(cd "$(dirname "$0")/.." && pwd)"
PGBASE="${PGBASE:-postgres://postgres@localhost}"
DB="$PGBASE/melihub_test"
for p in 8090 9933; do pid=$(ss -ltnp 2>/dev/null | grep ":$p " | grep -oP 'pid=\K[0-9]+' | head -1); [ -n "$pid" ] && kill "$pid"; done
sleep 1
psql "$PGBASE/postgres" -qc 'DROP DATABASE IF EXISTS melihub_test' -c 'CREATE DATABASE melihub_test' 2>&1 | grep -v NOTICE || true
(python3 -I "$DIR/scripts/ml_mock.py" >/dev/null 2>&1 &)
sleep 1
(cd "$DIR" && DATABASE_URL="$DB" JWT_SECRET=test_secret_0123456789abcdef PORT=8090 SYNC_INTERVAL_SECS=99999 \
  ML_CLIENT_ID=tc ML_CLIENT_SECRET=ts ML_REDIRECT_URI=http://localhost:8090/api/ml/callback \
  ML_API_BASE=http://127.0.0.1:9933 ML_AUTH_BASE=http://127.0.0.1:9933 \
  "$DIR/target/debug/melihub" >/tmp/melihub_smoke.log 2>&1 &)
sleep 2
python3 "$DIR/scripts/smoke_test.py"

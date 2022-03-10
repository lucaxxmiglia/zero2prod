#!/usr/bin/env bash
set -x
set -eo pipefail
DB_USER=xx
DB_PASSWORD="sxdcfvgb"
DB_NAME="newsletter"
DB_PORT="5433"

export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create
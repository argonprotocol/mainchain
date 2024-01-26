#!/bin/sh -e

ECHO "Initializing database... $DATABASE_URL"
cargo sqlx database drop && cargo sqlx database setup
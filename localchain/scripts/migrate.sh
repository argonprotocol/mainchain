# get current dir
CARGO_MANIFEST_DIR="$(cd "$(dirname "$0")/.." && pwd)"

export DATABASE_URL="sqlite:///${CARGO_MANIFEST_DIR}/test.db"

cargo sqlx migrate run && cargo sqlx prepare
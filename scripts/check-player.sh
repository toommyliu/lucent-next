#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
log=$(mktemp "${TMPDIR:-/tmp}/lucent-player.XXXXXX")
pid=
cleanup() {
    if [ -n "$pid" ]; then
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
    fi
}
trap cleanup EXIT
trap 'exit 130' INT TERM
cargo build -p lucent-app
RUST_LOG=lucent_host::player=debug "${CARGO_TARGET_DIR:-target}/debug/lucent" >"$log" 2>&1 &
pid=$!
started=$(date +%s)
while [ "$(( $(date +%s) - started ))" -lt 60 ]; do
    if line=$(LC_ALL=C grep -E 'fps=[1-9][0-9]* viewport=[0-9]+x[0-9]+@[0-9.]+' "$log" | head -n 1) && [ -n "$line" ]; then
        printf '%s\nLog: %s\n' "$line" "$log"
        exit 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
        cat "$log" >&2
        exit 1
    fi
    sleep 1
done
cat "$log" >&2
printf 'No rendered frame within 60 seconds. Log: %s\n' "$log" >&2
exit 1

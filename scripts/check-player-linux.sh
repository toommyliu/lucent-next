#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
shots=${1:-$(mktemp -d "${TMPDIR:-/tmp}/lucent-shots.XXXXXX")}
mkdir -p "$shots"
logs=$(mktemp -d "${TMPDIR:-/tmp}/lucent-check.XXXXXX")
export XDG_RUNTIME_DIR="$logs/runtime"
mkdir -m 700 "$XDG_RUNTIME_DIR"
screen=:97
pids=
cleanup() {
    for pid in $pids; do
        kill "$pid" 2>/dev/null || true
    done
    for pid in $pids; do
        wait "$pid" 2>/dev/null || true
    done
}
trap cleanup EXIT
trap 'exit 130' INT TERM

fail() {
    printf '%s Logs: %s\n' "$1" "$logs" >&2
    exit 1
}

await() {
    tries=0
    until "$@"; do
        tries=$((tries + 1))
        [ "$tries" -lt 100 ] || return 1
        sleep 0.1
    done
}

cargo build -p lucent-app --features overlay-check
bin="${CARGO_TARGET_DIR:-target}/debug/lucent"

Xvfb "$screen" -screen 0 1280x1024x24 >"$logs/xvfb.log" 2>&1 &
pids="$pids $!"
await test -S "/tmp/.X11-unix/X${screen#:}" || fail "Xvfb didn't start."
DISPLAY="$screen" weston --backend=x11 --renderer=pixman --socket=lucent-check \
    --width=1200 --height=950 --xwayland >"$logs/weston.log" 2>&1 &
pids="$pids $!"
await test -S "$XDG_RUNTIME_DIR/lucent-check" || fail "Weston didn't start."
await grep -q 'xserver listening on display' "$logs/weston.log" || fail "Xwayland didn't start."
xwayland=$(sed -n 's/.*xserver listening on display \(:[0-9]*\).*/\1/p' "$logs/weston.log" | head -n 1)

for mode in wayland x11; do
    log="$logs/lucent-$mode.log"
    if [ "$mode" = wayland ]; then
        env -u DISPLAY WAYLAND_DISPLAY=lucent-check RUST_LOG=lucent_host::player=debug \
            "$bin" >"$log" 2>&1 &
    else
        env -u WAYLAND_DISPLAY DISPLAY="$xwayland" RUST_LOG=lucent_host::player=debug \
            "$bin" >"$log" 2>&1 &
    fi
    app=$!
    pids="$pids $app"
    started=$(date +%s)
    until LC_ALL=C grep -Eq 'fps=([2-9]|[1-9][0-9]+) ' "$log"; do
        kill -0 "$app" 2>/dev/null || fail "Lucent exited on $mode."
        [ "$(($(date +%s) - started))" -lt 60 ] || fail "No frames shown on $mode within 60 seconds."
        sleep 1
    done
    sleep 15
    DISPLAY="$screen" import -window root "$shots/$mode.png"
    kill "$app"
    wait "$app" 2>/dev/null || true
    red=$(convert "$shots/$mode.png" -fill black +opaque '#ff0000' -fill white -opaque '#ff0000' \
        -format '%[fx:round(mean*w*h)]' info:)
    [ "$red" -ge 14400 ] || fail "The overlay box isn't above the game on $mode: $red red pixels."
    printf '%s: %s, %s red pixels, %s\n' "$mode" \
        "$(LC_ALL=C grep -Eo 'fps=[0-9]+ viewport=[^ ]+' "$log" | tail -n 1)" "$red" "$shots/$mode.png"
done

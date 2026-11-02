#!/usr/bin/env bash
# Real-window smoke test: builds the release app with the UI embedded, starts it, and checks the
# app's own exit code (not a dev runner's, which exits 0 even when the app aborts; spike E3).
# The app exits 0 once the UI reports the start screen (TRACEWRIGHT_SMOKE). Without a display,
# it runs under xvfb-run.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
timeout_s="${SMOKE_TIMEOUT_S:-120}"

(cd "$root/app/ui" && bun run build)
cargo build --manifest-path "$root/Cargo.toml" --locked --release -p tracewright-app \
  --features tauri/custom-protocol

data="$(mktemp -d)"
trap 'rm -rf "$data"' EXIT
log="$data/dev.tracewright.app/logs/tracewright.log"

run=(timeout --kill-after=10 "$timeout_s" "$root/target/release/tracewright-app")
if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
  # Xvfb has no DRI3, so WebKitGTK's DMA-BUF renderer can leave the page blank (E3 fallback).
  run=(env WEBKIT_DISABLE_DMABUF_RENDERER=1 xvfb-run --auto-servernum "${run[@]}")
fi

status=0
XDG_DATA_HOME="$data" TRACEWRIGHT_SMOKE=exit-when-start-screen-shown "${run[@]}" || status=$?

fail() {
  echo "smoke test failed: $1" >&2
  if [[ -f "$log" ]]; then
    echo "--- $log" >&2
    cat "$log" >&2
  fi
  exit 1
}
if [[ $status -eq 124 || $status -eq 137 ]]; then
  fail "the start screen was not reported within ${timeout_s}s"
fi
if [[ $status -ne 0 ]]; then
  fail "the app exited with $status"
fi
grep -q "UI ready: Start screen shown" "$log" || fail "the log has no 'UI ready' line"
echo "smoke test: the app showed the start screen and exited 0"

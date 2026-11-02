#!/usr/bin/env bash
# Fetches TAPAAL verifypn 4.2.3 for the differential corpus (NS §18 item 10) and prints the path
# of the checked binary. verifypn is GPL-3 and a development tool: it is downloaded into a cache
# outside the repository, never committed, linked or bundled (CODING_STANDARDS G-5).
#
#   tools/tapaal.sh            prints the path, downloading on first use
#
# The cache is $TAPAAL_CACHE_DIR, or $XDG_CACHE_HOME/tracewright/verifypn-4.2.3 (default
# ~/.cache/...). Both the release zip and the binary in it are pinned by SHA-256, and the binary is
# checked again on every run, so a changed cache is replaced instead of used. 4.2.3 is the newest
# release with a Linux binary (spike E1, docs/product/experiments/e1-state-space.md).
set -euo pipefail

version=4.2.3
url="https://github.com/TAPAAL/verifypn/releases/download/v${version}/verifypn-linux64.zip"
zip_sha256=de71aae9e476a8c65646d1214d8fbc563d774a1708fe1e8c51ce58f7351246f3
bin_sha256=afea1a80f31120b823e290fc17b6b156979d54557ac2ddac75472d2556a4e4b8

if [[ "$(uname -s)-$(uname -m)" != Linux-x86_64 ]]; then
  echo "tapaal: verifypn ${version} ships a binary for Linux x86_64 only" >&2
  exit 1
fi

cache="${TAPAAL_CACHE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/tracewright/verifypn-${version}}"
bin="$cache/verifypn-linux64"

checked() { echo "$2  $1" | sha256sum --check --status; }

if [[ -f $bin ]] && checked "$bin" "$bin_sha256"; then
  echo "$bin"
  exit 0
fi

download="$(mktemp -d)"
trap 'rm -rf "$download"' EXIT
echo "tapaal: downloading verifypn ${version}" >&2
curl --silent --show-error --location --fail --retry 3 --output "$download/verifypn.zip" "$url"
if ! checked "$download/verifypn.zip" "$zip_sha256"; then
  echo "tapaal: $url does not match the pinned SHA-256 $zip_sha256" >&2
  exit 1
fi
unzip -q "$download/verifypn.zip" verifypn-linux64 -d "$download"
if ! checked "$download/verifypn-linux64" "$bin_sha256"; then
  echo "tapaal: the binary in the zip does not match the pinned SHA-256 $bin_sha256" >&2
  exit 1
fi
chmod +x "$download/verifypn-linux64"
mkdir -p "$cache"
mv "$download/verifypn-linux64" "$bin"
echo "$bin"

#!/usr/bin/env bash
# The `check` command (CODING_STANDARDS G-2). CI runs each step as its own job with the same
# command, so a green local run means a green CI run.
#
#   tools/check.sh                 all steps
#   tools/check.sh rust ui         only these steps
#
# Steps: rust (fmt, clippy, tests, bindings and fixtures drift), arch (crate edges, net impl blocks,
# release profile, rule tests), ui (tsc, Oxlint, Oxfmt, import rules, Vitest, build), smoke (the
# real window, see tools/smoke.sh).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Written by `cargo test`: ts-rs bindings and the Rust-written IPC fixtures (IPC-2).
generated=(app/ui/src/ipc/bindings app/ui/src/ipc/fixtures)

ci="${GITHUB_ACTIONS:-false}"
step() { if [[ $ci == true ]]; then echo "::group::$*"; else echo "== $*"; fi; }
endstep() { if [[ $ci == true ]]; then echo "::endgroup::"; fi; }

rust() {
  step "rust: rustfmt"
  cargo fmt --all --check
  endstep
  step "rust: clippy"
  cargo clippy --workspace --all-targets --locked -- -D warnings
  endstep
  step "rust: tests (regenerating bindings and fixtures)"
  # Cleared first, so a binding whose type was removed shows up as a deleted file.
  rm -rf "${generated[@]}"
  mkdir -p "${generated[@]}"
  cargo test --workspace --locked
  endstep
  step "rust: generated bindings and fixtures match the commit"
  if [[ -n "$(git status --porcelain -- "${generated[@]}")" ]]; then
    git status --short -- "${generated[@]}"
    git --no-pager diff -- "${generated[@]}"
    echo "Generated IPC bindings or fixtures differ from the commit; commit them (IPC-2)." >&2
    exit 1
  fi
  endstep
}

arch() {
  step "arch: rule tests"
  bun test tools/
  app/ui/node_modules/.bin/tsc -p tools
  endstep
  step "arch: rules on the workspace"
  bun tools/arch/check.ts
  endstep
}

ui() {
  step "ui: format"
  app/ui/node_modules/.bin/oxfmt --check app/ui tools package.json
  endstep
  cd app/ui
  step "ui: typecheck"
  bun run typecheck
  endstep
  step "ui: lint"
  bun run lint
  endstep
  step "ui: import rules"
  bun run imports
  endstep
  step "ui: tests"
  bun run test
  endstep
  step "ui: build"
  bun run build
  endstep
  cd "$root"
}

smoke() {
  step "smoke: real window"
  tools/smoke.sh
  endstep
}

steps=("$@")
if [[ ${#steps[@]} -eq 0 ]]; then
  steps=(rust arch ui smoke)
fi
for s in "${steps[@]}"; do
  case "$s" in
    rust | arch | ui | smoke) "$s" ;;
    *)
      echo "unknown step: $s (steps: rust arch ui smoke)" >&2
      exit 2
      ;;
  esac
done
echo "check: ${steps[*]} passed"

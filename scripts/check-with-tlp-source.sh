#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Build a real TLP from source into a private prefix and let *its own* config parser judge what
# auto-tlp generates.  Catches renamed / deprecated / removed settings before users hit them.
#
#   scripts/check-with-tlp-source.sh [AUTO_TLP_BINARY] [TLP_GIT_REF]     (default: target/release/auto-tlp, main)
#
# TLP marks problems in `tlp-stat -c` output:
#   "#! ..."                 deprecated or removed parameter
#   "Runtime-translated ..." old (_ON_AC/_ON_BAT) name converted to the new (_ON_PRF/_ON_BAL) one
set -euo pipefail
BIN="$(realpath "${1:-target/release/auto-tlp}")"
REF="${2:-main}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "==> building TLP '$REF' from source"
git clone --quiet --depth 1 --branch "$REF" https://github.com/linrunner/TLP.git "$WORK/src"
ROOT="$WORK/root"
args=()
while read -r name dir; do args+=("TLP_${name}=${ROOT}${dir}"); done \
  < <(sed -n 's|^TLP_\([A-Z0-9]*\) *?= *\(/[^ ]*\).*|\1 \2|p' "$WORK/src/Makefile")
make -s -C "$WORK/src" "${args[@]}" install-tlp > /dev/null
mkdir -p "$ROOT/etc/tlp.d"
TLPSTAT="$ROOT/usr/bin/tlp-stat"
echo "==> using $("$TLPSTAT" -c 2>&1 | sed -n 's/^--- \(TLP [^ ]*\).*/\1/p' | head -n1)"

status=0
check() { # label, expect-modern(0/1), auto-tlp args...
  local label="$1" modern="$2"; shift 2
  "$BIN" --demo "$@" --terse > "$ROOT/etc/tlp.d/99-auto-tlp.conf" 2>/dev/null
  local out; out="$("$TLPSTAT" -c 2>&1 || true)"
  local ours; ours="$(grep '99-auto-tlp.conf' <<< "$out" || true)"
  local bad; bad="$(grep -F '#!' <<< "$ours" || true)"
  if [ -n "$bad" ]; then
    echo "::error::[$label] TLP reports deprecated/removed settings:"; echo "$bad"; status=1
  fi
  if [ "$modern" = 1 ]; then
    local tr; tr="$(grep -F 'Runtime-translated' <<< "$ours" || true)"
    if [ -n "$tr" ]; then
      echo "::error::[$label] old key names are still being emitted:"; echo "$tr"; status=1
    fi
    # every key we wrote must show up in TLP's effective configuration
    local missing=""
    while IFS= read -r key; do
      grep -q "99-auto-tlp.conf L[0-9]*: ${key}=" <<< "$ours" || missing="$missing $key"
    done < <(grep -oE '^[A-Z][A-Z0-9_]+=' "$ROOT/etc/tlp.d/99-auto-tlp.conf" | tr -d '=')
    if [ -n "$missing" ]; then
      echo "::error::[$label] TLP ignored these keys:$missing"; status=1
    fi
  fi
  [ "$status" = 0 ] && echo "ok: [$label] $(grep -c '' <<< "$ours") settings accepted"
  return 0
}

for p in balanced performance battery; do
  check "modern keys / $p" 1 --tlp-version 1.11 --profile "$p"
done
# Classic names must keep working, just with a "please rename" hint. Only deprecated/removed keys are errors.
check "classic keys / balanced" 0 --tlp-version 1.6 --profile balanced
exit "$status"

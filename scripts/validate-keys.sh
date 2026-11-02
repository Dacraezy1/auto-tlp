#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Check that every key auto-tlp emits (in every profile) is a real, documented TLP setting.
#
#   scripts/validate-keys.sh [AUTO_TLP_BINARY]
#
# Known keys come from the installed TLP (/etc/tlp.conf, /usr/share/tlp/defaults.conf) plus any
# files listed in TLP_KEY_FILES (space separated), e.g. a tlp.conf downloaded from upstream.
set -euo pipefail
BIN="${1:-target/release/auto-tlp}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

extract() { grep -hoE '^#?[[:space:]]*[A-Z][A-Z0-9_]+=' "$@" 2>/dev/null | tr -d '# =' | sort -u; }

# shellcheck disable=SC2086
extract /etc/tlp.conf /usr/share/tlp/defaults.conf ${TLP_KEY_FILES:-} > "$TMP/known"
if [ ! -s "$TMP/known" ]; then
  echo "error: no TLP key list found - is TLP installed?" >&2
  exit 1
fi
echo "known TLP keys: $(wc -l < "$TMP/known")"

status=0
for profile in balanced performance battery; do
  "$BIN" --profile "$profile" 2>/dev/null | grep -oE '^#?[A-Z][A-Z0-9_]+=' | tr -d '#=' | sort -u > "$TMP/ours"
  unknown="$(comm -23 "$TMP/ours" "$TMP/known" | tr '\n' ' ')"
  if [ -n "$unknown" ]; then
    echo "::error::profile '$profile' emits keys unknown to this TLP: $unknown"
    status=1
  else
    echo "profile '$profile': all $(wc -l < "$TMP/ours") keys are valid"
  fi
done
exit "$status"

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

# Missing files are fine (e.g. TLP not installed, only an upstream tlp.conf supplied): grep -s hides
# the error and `|| true` keeps `set -o pipefail` from aborting the script.
extract() { { grep -hsoE '^#?[[:space:]]*[A-Z][A-Z0-9_]+=' "$@" || true; } | tr -d '# =' | sort -u; }

# shellcheck disable=SC2086
extract /etc/tlp.conf /usr/share/tlp/defaults.conf ${TLP_KEY_FILES:-} > "$TMP/known"
if [ ! -s "$TMP/known" ]; then
  echo "error: no TLP key list found - is TLP installed?" >&2
  exit 1
fi
echo "known TLP keys: $(wc -l < "$TMP/known")"

# AUTO_TLP_EXTRA lets CI force a TLP version, e.g. AUTO_TLP_EXTRA="--tlp-version 1.11".
# "--demo" checks a full-featured sample laptop, because a CI runner has too little hardware
# (no battery, no GPU, no cpufreq) to exercise most of the keys.
status=0
for mode in "" "--demo"; do
  for profile in balanced performance battery; do
    # shellcheck disable=SC2086
    "$BIN" $mode --profile "$profile" ${AUTO_TLP_EXTRA:-} 2>/dev/null \
      | grep -oE '^#?[A-Z][A-Z0-9_]+=' | tr -d '#=' | sort -u > "$TMP/ours"
    unknown="$(comm -23 "$TMP/ours" "$TMP/known" | tr '\n' ' ')"
    label="${mode:-host scan} / $profile"
    if [ -n "$unknown" ]; then
      echo "::error::[$label] emits keys unknown to this TLP: $unknown"
      status=1
    else
      echo "[$label] all $(wc -l < "$TMP/ours") keys are valid"
    fi
  done
done
exit "$status"

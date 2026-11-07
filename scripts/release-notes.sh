#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Print the GitHub release notes for a version: its CHANGELOG.md section, then the install table.
#
#   scripts/release-notes.sh 0.2.0 > notes.md
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="${1:?usage: release-notes.sh X.Y.Z}"
VERSION="${VERSION#v}"

# Everything between "## [X.Y.Z]" and the next "## [" heading.
SECTION="$(awk -v v="$VERSION" '
  /^## \[/ { if (found) exit; if (index($0, "## [" v "]") == 1) { found = 1; next } }
  found { print }
' CHANGELOG.md)"

if [ -z "$(printf '%s' "$SECTION" | tr -d '[:space:]')" ]; then
  echo "error: CHANGELOG.md has no section for $VERSION (add '## [$VERSION] - YYYY-MM-DD')" >&2
  exit 1
fi

printf '## What'"'"'s new in %s\n%s\n' "$VERSION" "$SECTION"
cat <<'TABLE'

## Install

| Distro | How |
|---|---|
| Any Linux (x86_64 / aarch64) | `curl -fsSL https://raw.githubusercontent.com/Dacraezy1/auto-tlp/main/scripts/install.sh \| sh` |
| Debian, Ubuntu, Mint, Pop!_OS | `sudo apt install ./auto-tlp_*_amd64.deb` |
| Fedora, RHEL, openSUSE | `sudo dnf install ./auto-tlp-*.rpm` |
| Arch, Manjaro, EndeavourOS | `sudo pacman -U auto-tlp-*.pkg.tar.zst` |
| NixOS / Nix | `nix run github:Dacraezy1/auto-tlp` |
| Anything else | download the AppImage or the static `.tar.gz` |

Verify downloads with `sha256sum -c --ignore-missing SHA256SUMS`.
TABLE

#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Install the latest auto-tlp release (static binary) with checksum verification.
#
#   curl -fsSL https://raw.githubusercontent.com/Dacraezy1/auto-tlp/main/scripts/install.sh | sh
#
# Environment: PREFIX (default /usr/local), VERSION (default: latest, e.g. 0.1.0),
#              AUTO_TLP_BASE_URL (override the download location, used by the tests)
set -eu

REPO="Dacraezy1/auto-tlp"
PREFIX="${PREFIX:-/usr/local}"
VERSION="${VERSION:-latest}"

say() { printf '==> %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

case "$(uname -m)" in
  x86_64|amd64)  ARCH=x86_64 ;;
  aarch64|arm64) ARCH=aarch64 ;;
  *) die "unsupported CPU architecture: $(uname -m) (build from source: cargo install --git https://github.com/$REPO)" ;;
esac
[ "$(uname -s)" = "Linux" ] || die "auto-tlp only runs on Linux"

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL -o "$2" "$1"; }
  final_url() { curl -fsSLI -o /dev/null -w '%{url_effective}' "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO "$2" "$1"; }
  final_url() { wget -qS --spider "$1" 2>&1 | sed -n 's/^ *Location: //p' | tail -n1; }
else
  die "need curl or wget"
fi

if [ "$VERSION" = "latest" ]; then
  TAG="$(final_url "https://github.com/$REPO/releases/latest" | sed 's|.*/tag/||')"
  [ -n "$TAG" ] || die "could not determine the latest release"
else
  TAG="v${VERSION#v}"
fi
VER="${TAG#v}"
FILE="auto-tlp-$VER-$ARCH-linux.tar.gz"
BASE="${AUTO_TLP_BASE_URL:-https://github.com/$REPO/releases/download/$TAG}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
say "downloading $FILE"
fetch "$BASE/$FILE" "$TMP/$FILE"
fetch "$BASE/SHA256SUMS" "$TMP/SHA256SUMS"

say "verifying checksum"
( cd "$TMP" && grep " $FILE\$" SHA256SUMS | sha256sum -c - ) || die "checksum mismatch - aborting"

tar -C "$TMP" -xzf "$TMP/$FILE"
cd "$TMP/auto-tlp-$VER-$ARCH-linux"

SUDO=""
if [ "$(id -u)" != "0" ]; then
  command -v sudo >/dev/null 2>&1 || die "run as root or install sudo"
  SUDO="sudo"
fi
say "installing to $PREFIX"
$SUDO make install PREFIX="$PREFIX" >/dev/null
say "installed: $("$PREFIX/bin/auto-tlp" --version)"
echo "Try:  auto-tlp --report"

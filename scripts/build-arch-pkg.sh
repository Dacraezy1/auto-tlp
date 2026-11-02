#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Build (and smoke-test) the Arch Linux package from the current working tree.
# Must run as root inside an archlinux container:
#   docker run --rm -v "$PWD:/src" -v "$PWD/dist:/out" archlinux:base-devel /src/scripts/build-arch-pkg.sh /out
set -euo pipefail
OUT="${1:?usage: build-arch-pkg.sh OUT_DIR}"
SRC="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$SRC/Cargo.toml" | head -n1)"

pacman -Syu --noconfirm --needed base-devel git rust tlp >/dev/null
id builder >/dev/null 2>&1 || useradd -m builder

WORK=/home/builder/build
rm -rf "$WORK" && mkdir -p "$WORK"
tar -C "$SRC" --exclude=./target --exclude=./.git --exclude=./dist \
    --transform "s,^\.,auto-tlp-$VERSION," -czf "$WORK/auto-tlp-$VERSION.tar.gz" .
# Same PKGBUILD as the AUR one, but pointed at the local tarball.
sed -e 's|^source=.*|source=("auto-tlp-$pkgver.tar.gz")|' \
    -e "s|^sha256sums=.*|sha256sums=('SKIP')|" \
    "$SRC/packaging/arch/PKGBUILD" > "$WORK/PKGBUILD"
chown -R builder "$WORK"

su builder -c "cd $WORK && makepkg --noconfirm --cleanbuild"

mkdir -p "$OUT"
cp "$WORK"/auto-tlp-*.pkg.tar.zst "$OUT"/
pacman -U --noconfirm "$OUT"/auto-tlp-*.pkg.tar.zst
auto-tlp --version
auto-tlp --report >/dev/null
auto-tlp --terse | grep -q '^TLP_ENABLE='
test -f /usr/share/licenses/auto-tlp/LICENSE
echo "Arch package OK"

#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Bump the version everywhere:  scripts/bump-version.sh 0.2.0
set -euo pipefail
cd "$(dirname "$0")/.."
NEW="${1:?usage: bump-version.sh X.Y.Z}"
[[ "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "version must look like 1.2.3" >&2; exit 1; }
OLD="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"

sed -i "0,/^version = \"$OLD\"/s//version = \"$NEW\"/" Cargo.toml
sed -i "s/^pkgver=.*/pkgver=$NEW/; s/^pkgrel=.*/pkgrel=1/" packaging/arch/PKGBUILD
sed -i "s/^pkgver=.*/pkgver=$NEW/" packaging/alpine/APKBUILD
sed -i "s/^version=.*/version=$NEW/" packaging/void/template
sed -i "s/^Version:.*/Version:        $NEW/" packaging/rpm/auto-tlp.spec
if [ -f "packaging/gentoo/auto-tlp-$OLD.ebuild" ]; then
  git mv "packaging/gentoo/auto-tlp-$OLD.ebuild" "packaging/gentoo/auto-tlp-$NEW.ebuild" 2>/dev/null \
    || mv "packaging/gentoo/auto-tlp-$OLD.ebuild" "packaging/gentoo/auto-tlp-$NEW.ebuild"
fi
cargo update --workspace --offline 2>/dev/null || cargo check --offline >/dev/null

echo "Bumped $OLD -> $NEW. Next:"
echo "  1. add a section to CHANGELOG.md"
echo "  2. git commit -am 'Release v$NEW' && git tag v$NEW && git push --tags"
echo "  3. refresh checksums in packaging/{arch,alpine,void} after the release exists"

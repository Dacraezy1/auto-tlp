#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Build every release artifact for one target:  .tar.gz  .deb  .rpm  .AppImage
#
#   scripts/build-dist.sh [TARGET_TRIPLE] [OUT_DIR]
#
# Needs: rust (with TARGET installed), cargo-deb, cargo-generate-rpm.
# Set SKIP_APPIMAGE=1 to skip the AppImage (it downloads appimagetool).
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
OUT="${2:-dist}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"

case "$TARGET" in
  x86_64-*)  ARCH=x86_64;  DEB_ARCH=amd64 ;;
  aarch64-*) ARCH=aarch64; DEB_ARCH=arm64 ;;
  *) echo "unsupported target: $TARGET" >&2; exit 1 ;;
esac

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
echo "==> auto-tlp $VERSION for $TARGET ($ARCH)"

echo "==> build"
cargo build --release --locked --target "$TARGET"
BIN="target/$TARGET/release/auto-tlp"
"$BIN" --version

echo "==> tar.gz"
NAME="auto-tlp-$VERSION-$ARCH-linux"
STAGE="$(mktemp -d)/$NAME"
mkdir -p "$STAGE"
cp "$BIN" LICENSE README.md Makefile "$STAGE/"
cp -r man completions "$STAGE/"
tar -C "$(dirname "$STAGE")" --sort=name --owner=0 --group=0 --numeric-owner \
    -czf "$OUT/$NAME.tar.gz" "$NAME"

echo "==> deb"
cargo deb --target "$TARGET" --no-build --no-strip -o "$OUT/auto-tlp_${VERSION}_${DEB_ARCH}.deb"

echo "==> rpm"
cargo generate-rpm --target "$TARGET" --auto-req disabled -o "$OUT/auto-tlp-$VERSION-1.$ARCH.rpm"

if [ "${SKIP_APPIMAGE:-0}" != "1" ]; then
  echo "==> AppImage"
  TOOL="${APPIMAGETOOL:-}"
  if [ -z "$TOOL" ]; then
    TOOL="$(mktemp -d)/appimagetool"
    curl -fsSL -o "$TOOL" \
      "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$ARCH.AppImage"
    chmod +x "$TOOL"
  fi
  APPDIR="$(mktemp -d)/AppDir"
  install -Dm755 "$BIN" "$APPDIR/usr/bin/auto-tlp"
  install -Dm755 packaging/appimage/AppRun "$APPDIR/AppRun"
  install -Dm644 packaging/appimage/auto-tlp.desktop "$APPDIR/auto-tlp.desktop"
  install -Dm644 packaging/appimage/auto-tlp.svg "$APPDIR/auto-tlp.svg"
  ln -s auto-tlp.svg "$APPDIR/.DirIcon"
  install -Dm644 man/auto-tlp.1 "$APPDIR/usr/share/man/man1/auto-tlp.1"
  export ARCH APPIMAGE_EXTRACT_AND_RUN=1
  "$TOOL" --no-appstream "$APPDIR" "$OUT/auto-tlp-$VERSION-$ARCH.AppImage"
fi

echo "==> done"
ls -lh "$OUT"

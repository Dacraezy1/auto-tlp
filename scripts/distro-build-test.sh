#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Build and test auto-tlp with the *distro's own* Rust toolchain.
# Run inside a container with the repo mounted at /src:
#   docker run --rm -v "$PWD:/src" -w /src fedora:latest sh scripts/distro-build-test.sh
set -eu
# shellcheck disable=SC1091
. /etc/os-release
echo "==> ${PRETTY_NAME:-$ID}"
case "$ID $ID_LIKE" in
  *alpine*)   apk add --no-cache cargo gcc musl-dev ;;
  *fedora*)   dnf install -y -q cargo gcc ;;
  *suse*)     zypper --non-interactive install -y cargo gcc ;;
  *arch*)     pacman -Syu --noconfirm --needed rust gcc ;;
  *debian*|*ubuntu*) export DEBIAN_FRONTEND=noninteractive; apt-get update -qq && apt-get install -y -qq cargo gcc ;;
  *) echo "unsupported distro: $ID" >&2; exit 1 ;;
esac
rustc --version
export CARGO_TARGET_DIR=/tmp/target
cargo build --locked
cargo test --locked
/tmp/target/debug/auto-tlp --version
/tmp/target/debug/auto-tlp --report >/dev/null
echo "==> OK on ${PRETTY_NAME:-$ID}"

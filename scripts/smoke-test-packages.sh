#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Install the freshly built packages in clean containers of many distros and run them.
#   scripts/smoke-test-packages.sh [DIST_DIR]
set -euo pipefail
cd "$(dirname "$0")/.."
DIST="$(cd "${1:-dist}" && pwd)"

CHECK='auto-tlp --version \
 && auto-tlp --report >/dev/null \
 && auto-tlp --terse | grep -q "^TLP_ENABLE=" \
 && ls /usr/share/man/man1/auto-tlp.1* >/dev/null \
 && test -f /usr/share/bash-completion/completions/auto-tlp'

run() { # image, script
  echo "::group::$1"
  docker run --rm -v "$DIST:/pkgs:ro" "$1" sh -c "$2"
  echo "::endgroup::"
}

for img in debian:stable ubuntu:24.04; do
  run "$img" "export DEBIAN_FRONTEND=noninteractive; apt-get update -qq \
    && apt-get install -y -qq --no-install-recommends /pkgs/*.deb >/dev/null && $CHECK \
    && apt-get remove -y -qq auto-tlp >/dev/null && ! command -v auto-tlp"
done

run fedora:latest "dnf install -y -q /pkgs/*.rpm && $CHECK && dnf remove -y -q auto-tlp"
run opensuse/tumbleweed "zypper --non-interactive --no-gpg-checks install -y /pkgs/*.rpm >/dev/null && $CHECK"

# Enterprise Linux needs EPEL for the tlp dependency (x86_64 and aarch64 both exist).
run almalinux:9 "dnf install -y -q epel-release && dnf install -y -q /pkgs/*.rpm && $CHECK"

# The static tarball must run everywhere, including musl-based Alpine.
run alpine:latest "cd /tmp && tar xzf /pkgs/auto-tlp-*-linux.tar.gz && cd auto-tlp-* \
  && ./auto-tlp --version && ./auto-tlp --report >/dev/null \
  && apk add --no-cache make >/dev/null && make install DESTDIR=/tmp/root PREFIX=/usr >/dev/null \
  && test -x /tmp/root/usr/bin/auto-tlp && test -f /tmp/root/usr/share/man/man1/auto-tlp.1"

echo "All package smoke tests passed."

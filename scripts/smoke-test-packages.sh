#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Install the freshly built packages in clean containers of many distros and run them.
#   scripts/smoke-test-packages.sh [DIST_DIR]
#
# Notes on the checks:
#  * Container images often exclude documentation (Ubuntu "minimized", Fedora/Alma `nodocs`,
#    openSUSE `excludedocs`), so the man page is verified from the package's file list,
#    not from the filesystem.
#  * After removal we test the file, not `command -v`: dash/bash cache the path of a command
#    they already ran, so `command -v` would still "find" a binary that was deleted.
set -euo pipefail
cd "$(dirname "$0")/.."
DIST="$(cd "${1:-dist}" && pwd)"

BASE='auto-tlp --version \
 && auto-tlp --report >/dev/null \
 && auto-tlp --terse | grep -q "^TLP_ENABLE=" \
 && test -f /usr/share/bash-completion/completions/auto-tlp'
DEB_CHECK="$BASE && dpkg -L auto-tlp | grep -q /man1/auto-tlp.1"
RPM_CHECK="$BASE && rpm -q auto-tlp >/dev/null && rpm -ql auto-tlp | grep -q /man1/auto-tlp.1"

run() { # image, script
  echo "::group::$1"
  docker run --rm -v "$DIST:/pkgs:ro" "$1" sh -c "$2"
  echo "::endgroup::"
}

for img in debian:stable ubuntu:24.04; do
  run "$img" "export DEBIAN_FRONTEND=noninteractive; apt-get update -qq \
    && apt-get install -y -qq --no-install-recommends /pkgs/*.deb >/dev/null \
    && $DEB_CHECK \
    && apt-get remove -y -qq auto-tlp >/dev/null \
    && test ! -e /usr/bin/auto-tlp"
done

run fedora:latest "dnf install -y -q /pkgs/*.rpm && $RPM_CHECK"
run opensuse/tumbleweed "zypper --non-interactive --no-gpg-checks install -y /pkgs/*.rpm >/dev/null && $RPM_CHECK"

# Enterprise Linux needs EPEL for the tlp dependency (x86_64 and aarch64 both exist).
run almalinux:9 "dnf install -y -q epel-release && dnf install -y -q /pkgs/*.rpm && $RPM_CHECK"

# The static tarball must run everywhere, including musl-based Alpine.
run alpine:latest "cd /tmp && tar xzf /pkgs/auto-tlp-*-linux.tar.gz && cd auto-tlp-* \
  && ./auto-tlp --version && ./auto-tlp --report >/dev/null \
  && apk add --no-cache make >/dev/null && make install DESTDIR=/tmp/root PREFIX=/usr >/dev/null \
  && test -x /tmp/root/usr/bin/auto-tlp && test -f /tmp/root/usr/share/man/man1/auto-tlp.1"

echo "All package smoke tests passed."

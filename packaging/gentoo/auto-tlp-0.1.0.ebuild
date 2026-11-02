# Copyright 2026 Dacraezy1
# Distributed under the terms of the GNU General Public License v3 or later

EAPI=8

inherit cargo

DESCRIPTION="Scans your hardware and generates a clean, tailored TLP configuration"
HOMEPAGE="https://github.com/Dacraezy1/auto-tlp"
SRC_URI="https://github.com/Dacraezy1/auto-tlp/archive/v${PV}.tar.gz -> ${P}.tar.gz"

LICENSE="GPL-3+"
SLOT="0"
KEYWORDS="~amd64 ~arm64"

RDEPEND="sys-power/tlp"
BDEPEND=">=virtual/rust-1.75"

# auto-tlp has no crate dependencies, so no CRATES list is needed.

src_install() {
	cargo_src_install
	doman man/auto-tlp.1
	newbashcomp completions/auto-tlp.bash auto-tlp
	insinto /usr/share/zsh/site-functions
	doins completions/_auto-tlp
	insinto /usr/share/fish/vendor_completions.d
	doins completions/auto-tlp.fish
}

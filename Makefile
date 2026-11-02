# SPDX-License-Identifier: GPL-3.0-or-later
# Works from a git checkout (after `cargo build --release`) and from the release tarball.
PREFIX  ?= /usr/local
DESTDIR ?=
BIN     ?= $(firstword $(wildcard auto-tlp target/release/auto-tlp))

.PHONY: all install uninstall test
all:
	cargo build --release --locked

test:
	cargo test --locked

install:
	@test -n "$(BIN)" || { echo "binary not found - run 'cargo build --release' first"; exit 1; }
	install -Dm755 $(BIN) $(DESTDIR)$(PREFIX)/bin/auto-tlp
	install -Dm644 man/auto-tlp.1 $(DESTDIR)$(PREFIX)/share/man/man1/auto-tlp.1
	install -Dm644 completions/auto-tlp.bash $(DESTDIR)$(PREFIX)/share/bash-completion/completions/auto-tlp
	install -Dm644 completions/_auto-tlp $(DESTDIR)$(PREFIX)/share/zsh/site-functions/_auto-tlp
	install -Dm644 completions/auto-tlp.fish $(DESTDIR)$(PREFIX)/share/fish/vendor_completions.d/auto-tlp.fish
	install -Dm644 LICENSE $(DESTDIR)$(PREFIX)/share/licenses/auto-tlp/LICENSE

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/auto-tlp \
	      $(DESTDIR)$(PREFIX)/share/man/man1/auto-tlp.1 \
	      $(DESTDIR)$(PREFIX)/share/bash-completion/completions/auto-tlp \
	      $(DESTDIR)$(PREFIX)/share/zsh/site-functions/_auto-tlp \
	      $(DESTDIR)$(PREFIX)/share/fish/vendor_completions.d/auto-tlp.fish
	rm -rf $(DESTDIR)$(PREFIX)/share/licenses/auto-tlp

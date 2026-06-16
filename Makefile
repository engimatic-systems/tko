# Generated from tko.org. Do not edit by hand.

CARGO ?= cargo
EMACS ?= emacs
PREFIX ?= $(HOME)/.local
INSTALL_ROOT ?= $(PREFIX)
CARGO_INSTALL_FLAGS ?= --locked

TANGLE_OUTPUTS := .gitignore Cargo.toml Makefile src/lib.rs src/storage.rs src/query.rs src/read.rs src/write.rs src/lint.rs src/notes.rs src/cli.rs src/main.rs tests/cli.rs tests/storage.rs tests/read.rs tests/write.rs tests/lint.rs tests/notes.rs

.PHONY: tangle tangle-check check test install install-smoke

tangle:
	$(EMACS) --batch --eval "(progn (require 'org) (require 'ob-tangle) (setq org-confirm-babel-evaluate nil) (org-babel-tangle-file \"tko.org\"))"

tangle-check: tangle
	git diff --exit-code -- $(TANGLE_OUTPUTS)
	$(MAKE) check

check:
	$(CARGO) test

test:
	$(CARGO) test

install: check
	$(CARGO) install --path . --root $(INSTALL_ROOT) --force $(CARGO_INSTALL_FLAGS)

install-smoke:
	tmp="$$(mktemp -d)"; $(MAKE) install INSTALL_ROOT="$$tmp" CARGO_INSTALL_FLAGS="--locked --debug"; "$$tmp/bin/tko" help >/dev/null

# Generated from tko.org. Do not edit by hand.

CARGO ?= cargo
EMACS ?= emacs

TANGLE_OUTPUTS := .gitignore Cargo.toml Makefile src/lib.rs src/storage.rs src/query.rs src/read.rs src/write.rs src/lint.rs src/notes.rs src/cli.rs src/main.rs tests/cli.rs tests/storage.rs tests/read.rs tests/write.rs tests/lint.rs tests/notes.rs

.PHONY: tangle check test

tangle:
	$(EMACS) --batch --eval "(progn (require 'org) (require 'ob-tangle) (setq org-confirm-babel-evaluate nil) (org-babel-tangle-file \"tko.org\"))"

check: tangle
	git diff --exit-code -- $(TANGLE_OUTPUTS)
	$(CARGO) test

test:
	$(CARGO) test

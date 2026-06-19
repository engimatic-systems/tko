---
name: tko-ticket-discipline
description: Use when creating, updating, formatting, closing, or adding notes to tko tickets. Emphasizes readable Org structure, evidence-based closure, scoped ticket boundaries, code examples for code-facing features, and avoiding long single-line note dumps.
---

# TKO Ticket Discipline

Use this skill whenever editing `.tickets/*.org` or using `tko create`,
`tko add-note`, `tko close`, deps, tags, or status changes.

## Ground Rules

1. Read local project instructions and ticket conventions first.
2. Use `tko` for ids, status, deps, tags, listing, and lint.
3. Use manual file edits for substantial notes so Org stays readable.
4. Do not collapse design decisions, evidence, or examples into one long line.
5. Run `tko lint <id>` after ticket edits.

## Before Starting Ticket Work

When implementation or investigation is driven by a ticket, treat the ticket as
the system of record. Do not implement from the title, memory, or prior chat
alone.

1. Run `tko show --full <id>`.
2. Read the entire ticket, including Notes.
3. If the ticket has `TKO_PARENT`, run `tko show --full <parent>` and read it.
4. Read explicitly referenced docs or specs before editing files.
5. Check dependency state with `tko ready`, `tko blocked`, or direct ticket deps.
6. Confirm the ticket is actually actionable before changing code or state.

## Ticket Body Shape

Good tickets should be reviewable without conversation history:

- **Description**: durable change or decision.
- **Scope**: in scope and out of scope.
- **Acceptance Criteria**: evidence that closes the ticket.
- **Notes**: dated design refinements, evidence, risks, and examples.

Prefer one ticket per durable review surface or decision. Avoid tickets that are
only implementation steps unless the step produces a reviewable artifact.

## Note Formatting

Prefer this:

```org
** Notes

*** [2026-05-27 Wed 23:57Z] Design refinement

The importer should treat the input manifest as the contract boundary.

- Owns parsing `manifest.toml`.
- Reports missing required fields before writing output files.
- Keeps generated output deterministic for review.
- Records the source manifest path in evidence notes.

Changing the manifest format later should be a separate ticket because it alters
the author-facing API.
```

Avoid this:

```org
*** [timestamp] Design refinement: importer should treat manifest as contract boundary ... [200 words]
```

Heading is a title. Body carries prose, bullets, examples, and evidence.

## Code Examples

For code-facing tickets, include a short expected-use example when it clarifies
the feature. This lets the operator review semantics before implementation.

Example:

```org
** Acceptance Criteria

- The CLI exits successfully and writes JSON rows for matching items:

#+begin_src rust
use std::process::Command;

let output = Command::new("./target/debug/example-tool")
    .args(["list", "--format", "json", "--status", "open"])
    .output()
    .expect("example-tool should run");

assert!(output.status.success());

let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
assert!(stdout.lines().any(|line| line.contains(r#""status":"open""#)));
assert!(!stdout.lines().any(|line| line.contains(r#""status":"closed""#)));
#+end_src
```

Use examples to show intended API shape, not every implementation detail. Keep
examples small, concrete, and obviously non-secret.

## Evidence Notes

When work mutates code, docs, service config, data files, or live state, add
evidence as bullets:

- files changed or committed
- tests run and result
- build or release result
- live verification command
- migration or rollback note

Close tickets by evidence, not by intent.

## Manual Edit Bias

`tko add-note` is fine for short notes. For design notes, examples, or evidence
blocks, edit the ticket file with normal Org formatting, then run:

```sh
tko lint <id>
```

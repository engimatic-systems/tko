---
name: tko-basics
description: Use when someone is new to tko, asks what tko is or how to start, or needs the basic ticket lifecycle — the mental model and the minimal create→work→evidence→close loop. Defers ticket-writing rigor to tko-ticket-discipline and graph audits to tko-auditor.
---

# TKO Basics

First contact with tko. Goal: the mental model and the smallest working loop.
For writing good ticket bodies, use `tko-ticket-discipline`. For sequencing or
auditing a ticket graph, use `tko-auditor`.

## What tko is (the model)

- Tickets are the project's control surface — the durable memory between sessions.
- One ticket names **one durable change or decision**, not an implementation step.
- A ticket closes by **evidence** (command output, a committed file, a passing
  test), not by intent.
- Tickets are `.org` files under `.tickets/`, discovered from the current
  directory (or `TICKETS_DIR`). `tko init` creates the store.

## The loop

1. **Find work:** `tko ready` (actionable now) or `tko list` (everything). Output
   is one summary line per ticket: `id [status] :: title <- [deps]`.
2. **Read it whole:** `tko show --full <id>` — never work from the title alone.
3. **Start it:** `tko start <id>` (sets status `in_progress`).
4. **Record evidence as you go:** `tko add-note <id> --title "..." --body "..."`.
5. **Close by evidence:** `tko close <id>` once acceptance is met.

Other day-one commands: `tko create`, `tko status`, `tko dep` / `tko tag`,
`tko query`, `tko lint <id>`. Run `tko help` for the full list.

## A full cycle

```console
$ tko create "Pin netbird image tags" -t task -p 2
pla-ab12
$ tko start pla-ab12
Updated pla-ab12 -> in_progress
$ tko add-note pla-ab12 --title "Pinned" --body "compose pins server@v0.30, dashboard@v2.9"
Note added to pla-ab12
$ tko close pla-ab12
Updated pla-ab12 -> closed
```

## Finding things

```console
$ tko query status = open          # summary of open tickets (default output)
$ tko query --output id no deps    # just the ids of tickets with no deps
```

`tko query --help` documents the predicate grammar (`and`/`or`/`not`, `in`,
`contain`, `has`/`no`).

## When to graduate

- Writing a real Description / Scope / Acceptance, or closing with proper
  evidence → **tko-ticket-discipline**.
- Sequencing an epic, or checking a ticket graph stands on its own →
  **tko-auditor**.

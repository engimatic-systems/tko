# tko Specification

This document describes current `tko` behavior and the near-term compatibility
target for a systems-language rewrite. Behaviors marked Compatibility describe
the Bash implementation. Behaviors marked Stable describe the rewrite contract.

`tko` is a minimal ticket tracker backed by one Org file per ticket. It stores
ticket metadata in Org property drawers and ticket content in Org headings.

## Compatibility Status

Terms used in this spec:

- Stable: behavior expected to survive a rewrite.
- Compatibility: behavior observed in the Bash implementation. Preserve unless
  there is a deliberate migration.
- Planned: accepted direction, not implemented in the current Bash tool.

## Repository Discovery

Ticket files live in a `.tickets/` directory.

Discovery order:

1. If `TICKETS_DIR` is set, use it.
2. Otherwise search from the current working directory upward for `.tickets`.
3. Otherwise, for write commands that can initialize storage, use `.tickets` in
   the current working directory.
4. Otherwise fail.

Current write commands that can initialize storage:

- `create`

Compatibility: the missing-directory error still says `tk create`.

## Ticket File Format

Each ticket is stored as:

Canonical target format:

```org
:PROPERTIES:
:TKO_ID: pla-abcd
:TKO_STATUS: open
:TKO_DEPS: []
:TKO_LINKS: []
:TKO_CREATED: 2026-06-11T18:20:12Z
:TKO_TYPE: task
:TKO_PRIORITY: 2
:TKO_ASSIGNEE: rosin
:TKO_EXTERNAL_REF: gh-123
:TKO_PARENT: pla-parent
:TKO_TAGS: [repo/tko, tooling]
:END:

* Ticket title

** Description

Body text.
```

Stable properties:

- `TKO_ID`
- `TKO_STATUS`
- `TKO_DEPS`
- `TKO_LINKS`
- `TKO_CREATED`
- `TKO_TYPE`
- `TKO_PRIORITY`
- `TKO_ASSIGNEE`
- `TKO_EXTERNAL_REF`
- `TKO_PARENT`
- `TKO_TAGS`

Strict rule: normal `tko` commands read and write `TKO_*` keys only. Legacy
`TK_*` keys are not part of the Rust command contract.

List properties use bracketed comma-separated text:

```text
[]
[one]
[one, two]
```

They are not JSON in the Org file. The `query` command converts them to JSON
arrays.

If `TKO_ID` is missing, the filename stem is the ticket ID.

If a property drawer is missing during a property update, one is inserted at the
top of the file.

## Legacy Property Migration

The Rust train should include a small migration script or command that converts
legacy Bash `TK_*` properties to canonical `TKO_*` properties in existing
tickets.

Migration behavior:

- Operate on `.tickets/*.org` by default, with an option to target one file or
  ticket ID.
- Rename known `TK_*` keys to their `TKO_*` equivalents.
- Preserve property order, values, ticket body text, and newline style as much as
  practical.
- If only `TK_*` exists, replace it with `TKO_*`.
- If both `TKO_*` and `TK_*` exist for the same field and values match, remove
  the legacy `TK_*`.
- If both exist and values differ, keep both and report a conflict rather than
  guessing.
- Support dry-run/report mode.

This migration path is separate from normal command parsing. After migration,
legacy `TK_*` keys are `L004` lint failures if they remain in the active
property drawer.

## IDs

Generated IDs have this form:

```text
<prefix>-<hash>
```

`<prefix>` is derived from the current working directory basename:

- replace `-` and `_` with spaces
- use the first character of each word
- if fewer than two characters result, use the first three characters of the
  directory name

`<hash>` is four random lowercase ASCII letters or digits.

Examples:

- directory `planning` -> `pla-xxxx`
- directory `my-project` -> `mp-xxxx`

## Ticket Resolution

Commands that accept a ticket ID resolve it as follows:

1. Exact file match: `$TICKETS_DIR/<id>.org`.
2. Otherwise, substring match against ticket filename stems.
3. If exactly one file matches, use it.
4. If more than one file matches, fail with an ambiguous-ID error.
5. If none match, fail with a not-found error.

Stable: partial ID matching uses filename stems.

Compatibility: substring resolution is case-sensitive and uses filenames, not
`TKO_ID` property values.

## Status, Type, and Priority

Valid statuses:

- `open`
- `in_progress`
- `blocked`
- `closed`

Valid types:

- `bug`
- `feature`
- `task`
- `epic`
- `chore`

Valid priorities:

- `0`
- `1`
- `2`
- `3`
- `4`

Priority `0` is highest. Default priority is `2`.

Default type is `task`.

When displayed or queried, missing values default as follows:

- status: `open`
- deps: `[]`
- links: `[]`
- type: `task`
- priority: `2`
- tags: `[]`
- title: `Untitled` in list contexts

## Org Body Semantics

`tko` treats the first top-level Org heading as the ticket title.

`show` without `--full` prints only heading lines from the ticket body, after the
property drawer has been removed.

`show --full` prints the ticket body after the first property drawer has been
removed.

Semantic headings recognized by lint:

- `Description`
- `Scope`
- `Design`
- `Acceptance Criteria`
- `Notes`

Stable rule: semantic headings must occur at level 2 (`**`) and must not be
duplicated.

Compatibility: matching of semantic heading names is case-insensitive after
trimming trailing whitespace.

## Commands

### `help`

Usage:

```text
tko help
tko --help
tko -h
```

Prints command help and exits successfully.

### `create`

Usage:

```text
tko create [title] [options]
```

Options:

- `-d`, `--description <text>`
- `--scope <text>`
- `--design <text>`
- `--acceptance <text>`
- `-t`, `--type <bug|feature|task|epic|chore>`
- `-p`, `--priority <0|1|2|3|4>`
- `-a`, `--assignee <name>`
- `--external-ref <ref>`
- `--parent <ticket-id>`
- `--tags <comma-separated-tags>`

Behavior:

- Creates `$TICKETS_DIR` if needed.
- Generates a unique ID.
- Writes `$TICKETS_DIR/<id>.org`.
- Prints the new ID to stdout.
- Defaults title to `Untitled`.
- Defaults assignee from `git config user.name` when available.
- Resolves `--parent` through normal ticket resolution and stores the resolved
  filename stem.
- Converts escaped `\n` sequences in section options into real newlines.
- Writes only non-empty optional section bodies.
- Writes `TKO_TAGS` only when at least one tag is provided.

Compatibility:

- Unknown options fail.
- If multiple positional title arguments are passed, the last one wins.
- Tags are split on commas, trimmed, and empty items are dropped.

### `status`

Usage:

```text
tko status <id> <status>
```

Sets `TKO_STATUS` and prints:

```text
Updated <id> -> <status>
```

### `start`, `block`, `close`, `reopen`

Usage:

```text
tko start <id>
tko block <id>
tko close <id>
tko reopen <id>
```

Aliases for `status <id> in_progress`, `blocked`, `closed`, and `open`.

### `dep` and `undep`

Usage:

```text
tko dep <id> <dep-id>
tko undep <id> <dep-id>
```

Behavior:

- Resolve both IDs.
- Reject self-dependencies.
- Add or remove the dependency in `TKO_DEPS`.
- Preserve list order.
- Do not duplicate existing entries.

Output:

```text
Added dependency: <id> -> <dep-id>
Dependency already exists: <id> -> <dep-id>
Removed dependency: <id> -/-> <dep-id>
Dependency not present: <id> -/-> <dep-id>
```

### `link` and `unlink`

Usage:

```text
tko link <id> <target-id>
tko unlink <id> <target-id>
```

Behavior:

- Resolve both IDs.
- Reject self-links.
- Add or remove each ticket ID from the other's `TKO_LINKS`.
- Preserve list order.
- Do not duplicate existing entries.

Output:

```text
Added link: <id> <-> <target-id>
Link already exists: <id> <-> <target-id>
Removed link: <id> <-> <target-id>
Link not present: <id> <-> <target-id>
```

### `tag` and `untag`

Usage:

```text
tko tag <id> <tag> [tag...]
tko untag <id> <tag> [tag...]
```

Behavior:

- Add or remove tags from `TKO_TAGS`.
- Preserve list order.
- Do not duplicate existing entries.

Output:

```text
Added tag(s) to <id>: <tags...>
Tag(s) already present on <id>: <tags...>
Removed tag(s) from <id>: <tags...>
Tag(s) not present on <id>: <tags...>
```

### `ready`

Usage:

```text
tko ready [-a <assignee>] [-T <tag>]
tko ready [--assignee <assignee>] [--tag <tag>]
tko ready [--assignee=<assignee>] [--tag=<tag>]
```

Lists open or in-progress tickets whose dependencies are all closed.

Output is sorted by priority, then ID:

```text
<id padded to 8> [P<priority>][<status>] - <title>
```

Filters:

- `-a <assignee>`
- `--assignee <assignee>`
- `--assignee=<assignee>`
- `-T <tag>`
- `--tag <tag>`
- `--tag=<tag>`

Filter matching:

- Assignee matching is exact string equality against `TKO_ASSIGNEE`.
- Tag matching is exact string equality against one item in `TKO_TAGS`.
- Repeating the same filter is a usage error until multi-value semantics are
  explicitly designed.

Stable argument contract:

- Unknown flags are usage errors.
- Unexpected positional arguments are usage errors.
- Options that require values fail when the value is missing.

Compatibility: the Bash implementation ignores unknown arguments for `ready`.

### `blocked`

Usage:

```text
tko blocked [-a <assignee>] [-T <tag>]
tko blocked [--assignee <assignee>] [--tag <tag>]
tko blocked [--assignee=<assignee>] [--tag=<tag>]
```

Lists open or in-progress tickets with at least one dependency whose status is
not `closed`.

Output is sorted by priority, then ID:

```text
<id padded to 8> [P<priority>][<status>] - <title> <- [dep-a, dep-b]
```

Filters and argument handling match `ready`.

Compatibility: dependencies missing from the current ticket index count as
unresolved.

### `list` / `ls`

Usage:

```text
tko list [--status <status>] [-a <assignee>] [-T <tag>]
tko list [--status=<status>] [--assignee=<assignee>] [--tag=<tag>]
tko ls [--status <status>] [-a <assignee>] [-T <tag>]
tko ls [--status=<status>] [--assignee=<assignee>] [--tag=<tag>]
```

Lists tickets in filename sort order.

Output:

```text
<id padded to 8> [<status>] - <title>
<id padded to 8> [<status>] - <title> <- [dep-a, dep-b]
```

Filters:

- `--status <status>`
- `--status=<status>`
- `-a <assignee>`
- `--assignee <assignee>`
- `--assignee=<assignee>`
- `-T <tag>`
- `--tag <tag>`
- `--tag=<tag>`

Filter matching:

- Status matching is exact string equality after validating the supplied status.
- Assignee matching is exact string equality against `TKO_ASSIGNEE`.
- Tag matching is exact string equality against one item in `TKO_TAGS`.
- Repeating the same filter is a usage error until multi-value semantics are
  explicitly designed.

Stable argument contract:

- Unknown flags are usage errors.
- Unexpected positional arguments are usage errors.
- Options that require values fail when the value is missing.

Compatibility: the Bash implementation ignores unknown arguments for `list` and
`ls`.

### `show`

Usage:

```text
tko show [--full] <id>
tko show [-f] <id>
```

Metadata header:

```text
id: <id>
status: <status>
deps: <list>
links: <list>
created: <timestamp>
type: <type>
priority: <priority>
assignee: <assignee>        # only when non-empty
external-ref: <ref>         # only when non-empty
parent: <id>                # only when non-empty
tags: <list>
```

After a blank line:

- without `--full`, print body headings only
- with `--full`, print full body

If stdout is a TTY and `TICKET_PAGER` or `PAGER` is set, output is piped through
that pager.

Compatibility:

- The pager command is split on shell words by Bash `read -a`; shell quoting is
  not interpreted.
- `--full` and `-f` may appear before the ID.

### `add-note`

Usage:

```text
tko add-note <id> [note text]
```

Behavior:

- Reads note text from arguments, or from stdin when stdin is not a TTY.
- Converts escaped `\n` sequences into real newlines.
- Uses UTC timestamp format: `[YYYY-MM-DD Ddd HH:MMZ]`.
- Splits note text at the first newline.
- Uses the first line as the note heading text.
- Uses the remaining lines as note body.
- Appends the note under the semantic `** Notes` heading if one exists.
- If no `** Notes` heading exists, inserts `** Notes` after the first top-level
  ticket subtree.
- Individual note entries are always `***` headings.
- Note title enforcement applies before writing:
  - title text after the timestamp should be at most 50 characters
  - title text after the timestamp must be at most 72 characters
  - title prose beyond the limit belongs in the note body

Note heading format:

```org
*** [2026-06-11 Thu 18:20Z] Title line
Body line
```

If the first line is empty, the note heading is timestamp-only.

Output:

```text
Note added to <id>
```

Compatibility: the Bash implementation accepts long first lines and may match a
`Notes` heading at any level. The Rust target is stricter: `Notes` is always
`** Notes`, and note entries are always `***`.

### `query`

Usage:

```text
tko query [predicate]
```

Outputs one compact JSON object per ticket in filename sort order.

Default fields:

```json
{
  "id": "pla-abcd",
  "status": "open",
  "deps": [],
  "links": [],
  "created": "2026-06-11T18:20:12Z",
  "type": "task",
  "priority": "2",
  "tags": []
}
```

Optional fields appear only when non-empty:

- `assignee`
- `external-ref`
- `parent`

If a predicate is provided, only matching tickets are emitted.

The Rust target owns a small typed predicate DSL. It evaluates against the
ticket model, not against serialized JSON.

Predicate examples:

```text
status = open
status != closed
priority <= 2
type in [bug, feature]
status in [open, in_progress]
created >= 2026-06-01
assignee = rosin
parent = pla-root
external-ref = gh-123
tags contain repo/tko
deps contain pla-gq0a
links contain pla-abcd
has tags
no deps
has external-ref
no parent
status = open and priority <= 2
status in [open, in_progress] and tags contain repo/tko
(type = bug or priority = 0) and no deps
not tags contain archived
```

Fields:

- scalar string fields: `id`, `status`, `type`, `assignee`, `external-ref`,
  `parent`, `created`, `title`
- scalar numeric fields: `priority`
- plural string fields: `deps`, `links`, `tags`

Field names may contain hyphens. The predicate DSL has no subtraction operator,
so `external-ref` is parsed as one field name.

Grammar:

```text
expr        := or_expr
or_expr     := and_expr ("or" and_expr)*
and_expr    := not_expr ("and" not_expr)*
not_expr    := "not" not_expr | primary
primary     := comparison | membership | presence | "(" expr ")"

comparison  := scalar_field compare_op value
compare_op  := "=" | "!=" | "<" | "<=" | ">" | ">="

membership  := plural_field "contain" value
             | scalar_field "in" list

presence    := "has" field
             | "no" field

list        := "[" value ("," value)* "]"
```

Rules:

- No field aliases in v1. Use `tags`, `deps`, and `links`, not `tag`, `dep`, or
  `link`.
- `has <field>` means the field is present and non-empty.
- `no <field>` means the field is absent or empty.
- Empty lists count as `no <plural_field>`.
- Empty optional strings count as `no <scalar_field>`.
- `contain` is only valid for plural fields.
- `in` is only valid as scalar membership in a literal list.
- String values are bare tokens.
- Quoted strings are not part of the v1 DSL. Add them later only if real ticket
  data makes bare tokens too restrictive.
- `priority` comparisons are numeric.
- `created` comparisons are lexical comparisons over canonical timestamp/date
  strings.
- Unknown fields, type-incompatible operators, and malformed predicates are
  usage errors.

Compatibility:

- The Bash implementation treats the optional query argument as a `jq` filter.
- Rust `query` does not preserve jq compatibility.
- Bash JSON emits `priority` as a string. Rust JSON emits `priority` as a
  number.

### `lint`

Usage:

```text
tko lint [id-or-path]
```

Validates semantic heading conventions.

If a path exists, lint that path. Otherwise resolve the argument as a ticket ID.
With no argument, lint all tickets in filename sort order.

Current lint codes:

- `L001 duplicate semantic heading: <heading>`
- `L002 semantic heading must be level-2 (**): <heading>`

Planned lint codes:

- `L003 note title exceeds length target or hard limit`
- `L004 legacy TK_* property key remains after migration`

`L003` should warn above 50 characters and fail above 72 characters for note
title text after the timestamp. `add-note` should enforce the same hard limit at
write time.

`L004` fails when a known legacy `TK_*` property key remains in the active
property drawer after migration.

Output format:

```text
<file>:<line>: <code> <message>
```

Exit status is non-zero when any lint failure is found.

## Exit Status

Stable:

- successful commands exit `0`
- invalid usage exits non-zero
- missing or ambiguous tickets exit non-zero
- validation failures exit non-zero

Compatibility:

- Some usage errors return `1`; some lint resolution errors return `2`.
- External tool failures, such as `jq`, propagate their own status.

## Sorting

Ticket file iteration is lexicographic sort of `$TICKETS_DIR/*.org`.

`ready` and `blocked` sort by numeric priority, then ID.

## Dependencies

Current implementation depends on common Unix tools:

- Bash
- awk
- coreutils
- git for default assignee in `create`
- jq for Bash `query`
- rg or grep for internal property update checks

A rewrite should not require these tools for core behavior. Query filtering
uses the native typed predicate DSL.

## Planned Extensions

These features are planned and should be added to the spec when implemented.

### Note Table of Contents

Planned command:

```text
tko notes <id>
```

Expected behavior:

- list each Notes-section entry in document order
- print timestamp and title on one line
- do not print note bodies
- handle missing Notes sections
- mark timestamp-only notes clearly

### Note Title Length Enforcement

Planned lint code:

- `L003`

Expected behavior:

- warn when note title text after timestamp exceeds 50 characters
- fail when note title text after timestamp exceeds 72 characters
- treat overflow prose as body material, not heading material
- enforce the hard limit in both `lint` and `add-note`

### Selective Note Fetch

Planned command shape:

```text
tko show <id> --note <match>
```

Expected behavior:

- match note title case-insensitively
- timestamp matching is allowed if specified by final design
- print exactly one matching note subtree
- if ambiguous, list candidates rather than printing all bodies

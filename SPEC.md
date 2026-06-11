# tko Specification

This document describes `tko` as it exists today. It is the compatibility target
for a systems-language rewrite unless a behavior is explicitly marked planned or
deprecated here.

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

```org
:PROPERTIES:
:TK_ID: pla-abcd
:TK_STATUS: open
:TK_DEPS: []
:TK_LINKS: []
:TK_CREATED: 2026-06-11T18:20:12Z
:TK_TYPE: task
:TK_PRIORITY: 2
:TK_ASSIGNEE: rosin
:TK_EXTERNAL_REF: gh-123
:TK_PARENT: pla-parent
:TK_TAGS: [repo/tko, tooling]
:END:

* Ticket title

** Description

Body text.
```

Stable properties:

- `TK_ID`
- `TK_STATUS`
- `TK_DEPS`
- `TK_LINKS`
- `TK_CREATED`
- `TK_TYPE`
- `TK_PRIORITY`
- `TK_ASSIGNEE`
- `TK_EXTERNAL_REF`
- `TK_PARENT`
- `TK_TAGS`

List properties use bracketed comma-separated text:

```text
[]
[one]
[one, two]
```

They are not JSON in the Org file. The `query` command converts them to JSON
arrays.

If `TK_ID` is missing, the filename stem is the ticket ID.

If a property drawer is missing during a property update, one is inserted at the
top of the file.

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

Compatibility: substring resolution is case-sensitive and uses filenames, not
`TK_ID` property values.

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
- Writes `TK_TAGS` only when at least one tag is provided.

Compatibility:

- Unknown options fail.
- If multiple positional title arguments are passed, the last one wins.
- Tags are split on commas, trimmed, and empty items are dropped.

### `status`

Usage:

```text
tko status <id> <status>
```

Sets `TK_STATUS` and prints:

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
- Add or remove the dependency in `TK_DEPS`.
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
- Add or remove each ticket ID from the other's `TK_LINKS`.
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

- Add or remove tags from `TK_TAGS`.
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
```

Lists open or in-progress tickets whose dependencies are all closed.

Output is sorted by priority, then ID:

```text
<id padded to 8> [P<priority>][<status>] - <title>
```

Filters:

- `-a <assignee>`
- `--assignee=<assignee>`
- `-T <tag>`
- `--tag=<tag>`

Compatibility: unknown arguments are ignored.

### `blocked`

Usage:

```text
tko blocked [-a <assignee>] [-T <tag>]
```

Lists open or in-progress tickets with at least one dependency whose status is
not `closed`.

Output is sorted by priority, then ID:

```text
<id padded to 8> [P<priority>][<status>] - <title> <- [dep-a, dep-b]
```

Filters match `ready`.

Compatibility: dependencies missing from the current ticket index count as
unresolved.

### `list` / `ls`

Usage:

```text
tko list [--status=<status>] [-a <assignee>] [-T <tag>]
tko ls [--status=<status>] [-a <assignee>] [-T <tag>]
```

Lists tickets in filename sort order.

Output:

```text
<id padded to 8> [<status>] - <title>
<id padded to 8> [<status>] - <title> <- [dep-a, dep-b]
```

Filters:

- `--status=<status>`
- `-a <assignee>`
- `--assignee=<assignee>`
- `-T <tag>`
- `--tag=<tag>`

Compatibility: unknown arguments are ignored.

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
- Appends the note under the semantic `Notes` heading if one exists.
- If no `Notes` heading exists, inserts `** Notes` after the first top-level
  ticket subtree.
- If no top-level heading exists, appends `* Notes` at EOF.

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

Compatibility:

- Existing `Notes` heading lookup is case-insensitive and may match any Org
  heading level. New note entries are always inserted as `***`.
- Long first lines are currently accepted as full note heading text.

### `query`

Usage:

```text
tko query [jq-filter]
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

If a filter is provided, `query` pipes all objects through `jq -c <filter>`.

Compatibility:

- `jq` is required for all `query` usage, even without a filter.
- Bad filters fail with `jq`'s own error text and status.
- `priority` is a string in JSON output.

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
- jq for `query`
- rg or grep for internal property update checks

A rewrite should not require these tools for core behavior, except where the
command contract explicitly calls out external behavior such as `query`'s jq
filter language.

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

### Note Title Length Lint

Planned lint code:

- `L003`

Expected behavior:

- warn when note title text after timestamp exceeds 50 characters
- fail when note title text after timestamp exceeds 72 characters
- treat overflow prose as body material, not heading material

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

## Deliberate Rewrite Questions

Open questions before freezing a Rust implementation:

- Should partial ID matching continue to use filename stems only?
- Should unknown `ready`, `blocked`, and `list` arguments remain ignored?
- Should `query` keep embedding jq semantics, or should jq become an optional
  compatibility mode?
- Should note title length enforcement be warning/failure output in `lint`, in
  `add-note`, or both?
- Should note insertion preserve `Notes` heading depth by using child level
  `notes_level + 1` instead of always `***`?

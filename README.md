# tko

Minimal org-mode ticket system.

## What it is

tko tracks work as plain Org-mode files — one `.org` file per ticket under a
`.tickets/` directory you can read, grep, and commit alongside your code. It is a
single static binary: no daemon, no database, no network dependency.

The model is small:

- A ticket names **one durable change or decision**, with a status, optional
  dependencies, tags, and dated notes.
- Tickets are the project's control surface — durable memory between sessions.
- You close a ticket by **evidence** (a commit, command output, a passing test),
  not by intent.

Filtering uses a small typed predicate DSL (`tko query`) — no `jq` or external
query tool required.

## Install

Download the latest binary release from:

https://github.com/orgzeronine/tko/releases/latest

Linux x86_64:

```sh
mkdir -p "$HOME/.local/bin"
curl -L https://github.com/orgzeronine/tko/releases/latest/download/tko-x86_64-unknown-linux-musl.tar.gz |
  tar -xz -C "$HOME/.local/bin"
"$HOME/.local/bin/tko" help
```

Add `$HOME/.local/bin` to `PATH` if you want `tko` available in new shells.

Windows x86_64, from PowerShell:

```powershell
$InstallDir = "$env:LOCALAPPDATA\tko\bin"
New-Item -ItemType Directory -Force $InstallDir | Out-Null
$Zip = "$env:TEMP\tko.zip"
Invoke-WebRequest -Uri "https://github.com/orgzeronine/tko/releases/latest/download/tko-x86_64-pc-windows-msvc.zip" -OutFile $Zip
Expand-Archive -Path $Zip -DestinationPath $InstallDir -Force
& "$InstallDir\tko.exe" help
```

Add `$InstallDir` to `PATH` if you want `tko` available in new shells.

## Usage

Create a store and walk the basic loop:

```sh
tko init                                   # create ./.tickets
id=$(tko create "Pin image tags" -t task -p 2)
tko ready                                  # what's actionable now
tko show --full "$id"                      # read the whole ticket
tko start "$id"                            # status -> in_progress
tko add-note "$id" --title "Pinned" --body "compose pins server@v0.30"
tko close "$id"                            # close by evidence
```

Filter with the predicate DSL (quote predicates containing `<` `>` `(` `)`):

```sh
tko query status = open                       # default output: summary lines
tko query 'status in [open, in_progress]'     # membership
tko query 'priority <= 2 and has assignee'    # compare + presence, combined
tko query 'has parent and no assignee'        # presence / absence
tko query --output id no deps                 # ids only, for pipelines
```

Grammar (keywords are case-sensitive):

```text
FIELD OP VALUE         OP: = != < <= > >=   (priority compares numerically)
FIELD contain VALUE    membership in a plural field (deps, links, tags)
FIELD in [A, B, C]     scalar field equals any listed value
has FIELD / no FIELD   field present / absent
and  or  not  ( )       combine and group
```

- **Scalar fields:** `id` `status` `type` `assignee` `external-ref` `parent` `created` `title` `priority`
- **Plural fields:** `deps` `links` `tags`

Run `tko help` for all commands; `tko query --help` documents this same grammar.

## Build From Source

```sh
cargo test --locked
cargo build --release --locked
```

## Release

Push a version tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The release workflow builds Linux and Windows binaries, uploads them as workflow artifacts, creates a GitHub Release for the tag, and attaches both archives plus `SHA256SUMS`.

#!/usr/bin/env bash
set -euo pipefail

# ticket - minimal org-mode ticket tracker
# Stores one ticket per file in .tickets/*.org using buffer-level TK_* properties.

find_tickets_dir() {
  if [ -n "${TICKETS_DIR:-}" ]; then
    printf '%s\n' "$TICKETS_DIR"
    return 0
  fi

  local dir="$PWD"
  while [ "$dir" != "/" ]; do
    if [ -d "$dir/.tickets" ]; then
      printf '%s\n' "$dir/.tickets"
      return 0
    fi
    dir=$(dirname "$dir")
  done

  if [ -d "/.tickets" ]; then
    printf '%s\n' "/.tickets"
    return 0
  fi

  return 1
}

WRITE_COMMANDS="create"

init_tickets_dir() {
  local cmd="$1"
  local is_write=0
  case " $WRITE_COMMANDS " in
    *" $cmd "*) is_write=1 ;;
  esac

  if TICKETS_DIR=$(find_tickets_dir); then
    if [ "$is_write" -eq 0 ] && [ ! -d "$TICKETS_DIR" ]; then
      echo "Error: tickets directory '$TICKETS_DIR' does not exist" >&2
      return 1
    fi
    return 0
  fi

  if [ "$is_write" -eq 1 ]; then
    TICKETS_DIR=".tickets"
    return 0
  fi

  echo "Error: no .tickets directory found (searched parent directories)" >&2
  echo "Run 'tk create' to initialize, or set TICKETS_DIR env var" >&2
  return 1
}

TICKET_PAGER="${TICKET_PAGER:-${PAGER:-}}"

if command -v rg >/dev/null 2>&1; then
  _grep() { rg "$@"; }
else
  _grep() { grep "$@"; }
fi

trim() {
  local value="$*"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s\n' "$value"
}

_iso_date() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

_note_timestamp() {
  date -u +"[%Y-%m-%d %a %H:%MZ]"
}

normalize_escaped_newlines() {
  printf '%s' "${1//\\n/$'\n'}"
}

ensure_dir() {
  mkdir -p "$TICKETS_DIR"
}

generate_id() {
  local dir_name prefix hash
  dir_name=$(basename "$(pwd)")
  prefix=$(echo "$dir_name" | sed 's/[-_]/ /g' | awk '{for(i=1;i<=NF;i++) printf substr($i,1,1)}')
  if [ "${#prefix}" -lt 2 ]; then
    prefix="${dir_name:0:3}"
  fi
  hash=$(LC_ALL=C tr -dc 'a-z0-9' < /dev/urandom | head -c 4)
  printf '%s-%s\n' "$prefix" "$hash"
}

ticket_files_sorted() {
  shopt -s nullglob
  local files=("$TICKETS_DIR"/*.org)
  shopt -u nullglob

  if [ "${#files[@]}" -eq 0 ]; then
    return 0
  fi

  printf '%s\n' "${files[@]}" | sort
}

ticket_id_from_file() {
  local file="$1" id
  id=$(ticket_field "$file" "TK_ID")
  if [ -n "$id" ]; then
    printf '%s\n' "$id"
  else
    basename "$file" .org
  fi
}

ticket_path() {
  local id="$1"
  read -r id <<< "$id"

  local exact="$TICKETS_DIR/${id}.org"
  if [ -f "$exact" ]; then
    printf '%s\n' "$exact"
    return 0
  fi

  local -a matches=()
  local file base
  while IFS= read -r file; do
    [ -n "$file" ] || continue
    base=$(basename "$file" .org)
    case "$base" in
      *"$id"*) matches+=("$file") ;;
    esac
  done < <(ticket_files_sorted)

  if [ "${#matches[@]}" -eq 1 ]; then
    printf '%s\n' "${matches[0]}"
    return 0
  fi

  if [ "${#matches[@]}" -gt 1 ]; then
    echo "Error: ambiguous ID '$id' matches multiple tickets" >&2
    return 1
  fi

  echo "Error: ticket '$id' not found" >&2
  return 1
}

properties_tsv() {
  local file="$1"
  awk '
  BEGIN { in_props = 0; seen = 0 }
  $0 == ":PROPERTIES:" && seen == 0 { in_props = 1; seen = 1; next }
  in_props && $0 == ":END:" { exit }
  in_props {
    if ($0 ~ /^:[A-Za-z0-9_]+:/) {
      line = $0
      sub(/^:/, "", line)
      key = line
      sub(/:.*/, "", key)
      value = $0
      sub(/^:[A-Za-z0-9_]+:[[:space:]]*/, "", value)
      printf "%s\t%s\n", key, value
    }
  }
  ' "$file"
}

ticket_field() {
  local file="$1" key="$2" out
  out=$(properties_tsv "$file" | awk -F'\t' -v key="$key" '$1 == key { print $2; found = 1; exit } END { if (!found) exit 1 }' 2>/dev/null || true)
  printf '%s\n' "$out"
}

upsert_property() {
  local file="$1" key="$2" value="$3"
  local tmp has_props

  has_props=false
  if _grep -q '^:PROPERTIES:$' "$file"; then
    has_props=true
  fi

  tmp=$(mktemp)

  if [ "$has_props" = true ]; then
    awk -v key="$key" -v value="$value" '
    BEGIN { in_props = 0; updated = 0 }
    $0 == ":PROPERTIES:" && in_props == 0 { in_props = 1; print; next }
    in_props && $0 == ":END:" {
      if (!updated) {
        printf ":%s: %s\n", key, value
      }
      in_props = 0
      print
      next
    }
    in_props && $0 ~ ("^:" key ":[[:space:]]*") {
      printf ":%s: %s\n", key, value
      updated = 1
      next
    }
    { print }
    ' "$file" > "$tmp"
  else
    {
      echo ":PROPERTIES:"
      printf ':%s: %s\n' "$key" "$value"
      echo ":END:"
      cat "$file"
    } > "$tmp"
  fi

  mv "$tmp" "$file"
}

ticket_title() {
  local file="$1"
  awk '/^\*+[[:space:]]+/ { sub(/^\*+[[:space:]]+/, "", $0); print; exit }' "$file"
}

ticket_body() {
  local file="$1"
  awk '
  BEGIN { in_props = 0; consumed = 0 }
  $0 == ":PROPERTIES:" && consumed == 0 { in_props = 1; consumed = 1; next }
  in_props && $0 == ":END:" { in_props = 0; next }
  in_props { next }
  { print }
  ' "$file"
}

ticket_outline() {
  local file="$1"
  ticket_body "$file" | awk '/^\*+[[:space:]]+/ { print }'
}

list_items() {
  local list="$1"
  list=$(trim "$list")
  list="${list#[}"
  list="${list%]}"

  if [ -z "$(printf '%s' "$list" | tr -d '[:space:]')" ]; then
    return 0
  fi

  printf '%s\n' "$list" | tr ',' '\n' | while IFS= read -r item; do
    item=$(trim "$item")
    if [ -n "$item" ]; then
      printf '%s\n' "$item"
    fi
  done
}

list_contains() {
  local list="$1" needle="$2" item
  while IFS= read -r item; do
    if [ "$item" = "$needle" ]; then
      return 0
    fi
  done < <(list_items "$list")
  return 1
}

list_from_items() {
  local -a items=("$@")
  local out="" item

  for item in "${items[@]}"; do
    item=$(trim "$item")
    if [ -z "$item" ]; then
      continue
    fi
    if [ -n "$out" ]; then
      out="$out, $item"
    else
      out="$item"
    fi
  done

  if [ -z "$out" ]; then
    printf '[]\n'
  else
    printf '[%s]\n' "$out"
  fi
}

normalize_list_bracketed() {
  local list="$1"
  local -a items=()
  local item
  while IFS= read -r item; do
    items+=("$item")
  done < <(list_items "$list")
  list_from_items "${items[@]}"
}

ticket_list_field() {
  local file="$1" key="$2"
  local value
  value=$(ticket_field "$file" "$key")
  if [ -z "$value" ]; then
    printf '[]\n'
    return 0
  fi
  normalize_list_bracketed "$value"
}

LIST_MUTATED=false
LIST_RESULT="[]"

list_add_unique_preserve_order() {
  local list="$1" item="$2"
  local -a values=()
  local value found=false

  while IFS= read -r value; do
    values+=("$value")
    if [ "$value" = "$item" ]; then
      found=true
    fi
  done < <(list_items "$list")

  if [ "$found" = true ]; then
    LIST_MUTATED=false
    LIST_RESULT=$(list_from_items "${values[@]}")
    return 0
  fi

  values+=("$item")
  LIST_MUTATED=true
  LIST_RESULT=$(list_from_items "${values[@]}")
}

list_remove_preserve_order() {
  local list="$1" item="$2"
  local -a values=()
  local value removed=false

  while IFS= read -r value; do
    if [ "$value" = "$item" ]; then
      removed=true
      continue
    fi
    values+=("$value")
  done < <(list_items "$list")

  if [ "$removed" = true ]; then
    LIST_MUTATED=true
  else
    LIST_MUTATED=false
  fi
  LIST_RESULT=$(list_from_items "${values[@]}")
}

csv_to_list() {
  local csv="$1"
  if [ -z "$(trim "$csv")" ]; then
    printf '[]\n'
    return 0
  fi

  local -a pieces=()
  IFS=',' read -r -a pieces <<< "$csv"

  local out="" item
  for item in "${pieces[@]}"; do
    item=$(trim "$item")
    if [ -z "$item" ]; then
      continue
    fi
    if [ -n "$out" ]; then
      out="$out, $item"
    else
      out="$item"
    fi
  done

  if [ -z "$out" ]; then
    printf '[]\n'
  else
    printf '[%s]\n' "$out"
  fi
}

list_to_json() {
  local list="$1"
  jq -Rn --arg s "$list" '
    ($s
      | sub("^\\["; "")
      | sub("\\]$"; "")
      | split(",")
      | map(gsub("^\\s+|\\s+$"; ""))
      | map(select(length > 0))
    )
  '
}

validate_status() {
  local status="$1"
  case "$status" in
    open|in_progress|blocked|closed) return 0 ;;
    *)
      echo "Error: invalid status '$status'. Must be one of: open in_progress blocked closed" >&2
      return 1
      ;;
  esac
}

validate_type() {
  local issue_type="$1"
  case "$issue_type" in
    bug|feature|task|epic|chore) return 0 ;;
    *)
      echo "Error: invalid type '$issue_type'. Must be one of: bug feature task epic chore" >&2
      return 1
      ;;
  esac
}

validate_priority() {
  local priority="$1"
  case "$priority" in
    0|1|2|3|4) return 0 ;;
    *)
      echo "Error: invalid priority '$priority'. Must be 0-4" >&2
      return 1
      ;;
  esac
}

write_section() {
  local heading="$1" content="$2"
  if [ -z "$content" ]; then
    return 0
  fi

  printf '** %s\n\n%s\n\n' "$heading" "$content"
}

cmd_create() {
  ensure_dir

  local title="" description="" scope="" design="" acceptance=""
  local priority=2 issue_type="task" assignee="" external_ref="" parent="" tags=""

  assignee=$(git config user.name 2>/dev/null || true)

  while [ "$#" -gt 0 ]; do
    case "$1" in
      -d|--description)
        [ "$#" -ge 2 ] || { echo "Error: $1 requires a value" >&2; return 1; }
        description="$2"
        shift 2
        ;;
      --scope)
        [ "$#" -ge 2 ] || { echo "Error: $1 requires a value" >&2; return 1; }
        scope="$2"
        shift 2
        ;;
      --design)
        [ "$#" -ge 2 ] || { echo "Error: $1 requires a value" >&2; return 1; }
        design="$2"
        shift 2
        ;;
      --acceptance)
        [ "$#" -ge 2 ] || { echo "Error: $1 requires a value" >&2; return 1; }
        acceptance="$2"
        shift 2
        ;;
      -p|--priority)
        [ "$#" -ge 2 ] || { echo "Error: $1 requires a value" >&2; return 1; }
        priority="$2"
        shift 2
        ;;
      -t|--type)
        [ "$#" -ge 2 ] || { echo "Error: $1 requires a value" >&2; return 1; }
        issue_type="$2"
        shift 2
        ;;
      -a|--assignee)
        [ "$#" -ge 2 ] || { echo "Error: $1 requires a value" >&2; return 1; }
        assignee="$2"
        shift 2
        ;;
      --external-ref)
        [ "$#" -ge 2 ] || { echo "Error: $1 requires a value" >&2; return 1; }
        external_ref="$2"
        shift 2
        ;;
      --parent)
        [ "$#" -ge 2 ] || { echo "Error: $1 requires a value" >&2; return 1; }
        parent="$2"
        shift 2
        ;;
      --tags)
        [ "$#" -ge 2 ] || { echo "Error: $1 requires a value" >&2; return 1; }
        tags="$2"
        shift 2
        ;;
      -*)
        echo "Unknown option: $1" >&2
        return 1
        ;;
      *)
        title="$1"
        shift
        ;;
    esac
  done

  validate_type "$issue_type" || return 1
  validate_priority "$priority" || return 1

  if [ -n "$parent" ]; then
    local parent_file
    parent_file=$(ticket_path "$parent") || return 1
    parent=$(basename "$parent_file" .org)
  fi

  description=$(normalize_escaped_newlines "$description")
  scope=$(normalize_escaped_newlines "$scope")
  design=$(normalize_escaped_newlines "$design")
  acceptance=$(normalize_escaped_newlines "$acceptance")

  local tags_list
  tags_list=$(csv_to_list "$tags")

  title="${title:-Untitled}"

  local id file now
  while :; do
    id=$(generate_id)
    file="$TICKETS_DIR/${id}.org"
    [ ! -f "$file" ] && break
  done
  now=$(_iso_date)

  {
    echo ":PROPERTIES:"
    printf ':TK_ID: %s\n' "$id"
    echo ':TK_STATUS: open'
    echo ':TK_DEPS: []'
    echo ':TK_LINKS: []'
    printf ':TK_CREATED: %s\n' "$now"
    printf ':TK_TYPE: %s\n' "$issue_type"
    printf ':TK_PRIORITY: %s\n' "$priority"
    if [ -n "$assignee" ]; then
      printf ':TK_ASSIGNEE: %s\n' "$assignee"
    fi
    if [ -n "$external_ref" ]; then
      printf ':TK_EXTERNAL_REF: %s\n' "$external_ref"
    fi
    if [ -n "$parent" ]; then
      printf ':TK_PARENT: %s\n' "$parent"
    fi
    if [ "$tags_list" != '[]' ]; then
      printf ':TK_TAGS: %s\n' "$tags_list"
    fi
    echo ':END:'
    echo
    printf '* %s\n\n' "$title"
    write_section 'Description' "$description"
    write_section 'Scope' "$scope"
    write_section 'Design' "$design"
    write_section 'Acceptance Criteria' "$acceptance"
  } > "$file"

  printf '%s\n' "$id"
}

cmd_status() {
  if [ "$#" -lt 2 ]; then
    echo "Usage: $(basename "$0") status <id> <status>" >&2
    echo "Valid statuses: open in_progress blocked closed" >&2
    return 1
  fi

  local id="$1" status="$2" file full_id
  validate_status "$status" || return 1

  file=$(ticket_path "$id") || return 1
  upsert_property "$file" "TK_STATUS" "$status"
  full_id=$(ticket_id_from_file "$file")
  printf 'Updated %s -> %s\n' "$full_id" "$status"
}

cmd_start() {
  if [ "$#" -lt 1 ]; then
    echo "Usage: $(basename "$0") start <id>" >&2
    return 1
  fi
  cmd_status "$1" "in_progress"
}

cmd_block() {
  if [ "$#" -lt 1 ]; then
    echo "Usage: $(basename "$0") block <id>" >&2
    return 1
  fi
  cmd_status "$1" "blocked"
}

cmd_close() {
  if [ "$#" -lt 1 ]; then
    echo "Usage: $(basename "$0") close <id>" >&2
    return 1
  fi
  cmd_status "$1" "closed"
}

cmd_reopen() {
  if [ "$#" -lt 1 ]; then
    echo "Usage: $(basename "$0") reopen <id>" >&2
    return 1
  fi
  cmd_status "$1" "open"
}

cmd_dep() {
  if [ "$#" -lt 2 ]; then
    echo "Usage: $(basename "$0") dep <id> <dep-id>" >&2
    return 1
  fi

  local file dep_file id dep_id deps new_deps
  file=$(ticket_path "$1") || return 1
  dep_file=$(ticket_path "$2") || return 1
  id=$(ticket_id_from_file "$file")
  dep_id=$(ticket_id_from_file "$dep_file")

  if [ "$id" = "$dep_id" ]; then
    echo "Error: self dependency is not allowed: $id" >&2
    return 1
  fi

  deps=$(ticket_list_field "$file" "TK_DEPS")
  list_add_unique_preserve_order "$deps" "$dep_id"
  new_deps="$LIST_RESULT"
  if [ "$LIST_MUTATED" = true ]; then
    upsert_property "$file" "TK_DEPS" "$new_deps"
    printf 'Added dependency: %s -> %s\n' "$id" "$dep_id"
  else
    printf 'Dependency already exists: %s -> %s\n' "$id" "$dep_id"
  fi
}

cmd_undep() {
  if [ "$#" -lt 2 ]; then
    echo "Usage: $(basename "$0") undep <id> <dep-id>" >&2
    return 1
  fi

  local file dep_file id dep_id deps new_deps
  file=$(ticket_path "$1") || return 1
  dep_file=$(ticket_path "$2") || return 1
  id=$(ticket_id_from_file "$file")
  dep_id=$(ticket_id_from_file "$dep_file")

  if [ "$id" = "$dep_id" ]; then
    echo "Error: self dependency is not allowed: $id" >&2
    return 1
  fi

  deps=$(ticket_list_field "$file" "TK_DEPS")
  list_remove_preserve_order "$deps" "$dep_id"
  new_deps="$LIST_RESULT"
  if [ "$LIST_MUTATED" = true ]; then
    upsert_property "$file" "TK_DEPS" "$new_deps"
    printf 'Removed dependency: %s -/-> %s\n' "$id" "$dep_id"
  else
    printf 'Dependency not present: %s -/-> %s\n' "$id" "$dep_id"
  fi
}

cmd_link() {
  if [ "$#" -lt 2 ]; then
    echo "Usage: $(basename "$0") link <id> <target-id>" >&2
    return 1
  fi

  local file target_file id target_id
  local links_a links_b new_links_a new_links_b
  local changed_a=false changed_b=false

  file=$(ticket_path "$1") || return 1
  target_file=$(ticket_path "$2") || return 1
  id=$(ticket_id_from_file "$file")
  target_id=$(ticket_id_from_file "$target_file")

  if [ "$id" = "$target_id" ]; then
    echo "Error: self link is not allowed: $id" >&2
    return 1
  fi

  links_a=$(ticket_list_field "$file" "TK_LINKS")
  list_add_unique_preserve_order "$links_a" "$target_id"
  new_links_a="$LIST_RESULT"
  if [ "$LIST_MUTATED" = true ]; then
    changed_a=true
  fi

  links_b=$(ticket_list_field "$target_file" "TK_LINKS")
  list_add_unique_preserve_order "$links_b" "$id"
  new_links_b="$LIST_RESULT"
  if [ "$LIST_MUTATED" = true ]; then
    changed_b=true
  fi

  if [ "$changed_a" = true ]; then
    upsert_property "$file" "TK_LINKS" "$new_links_a"
  fi
  if [ "$changed_b" = true ]; then
    upsert_property "$target_file" "TK_LINKS" "$new_links_b"
  fi

  if [ "$changed_a" = true ] || [ "$changed_b" = true ]; then
    printf 'Added link: %s <-> %s\n' "$id" "$target_id"
  else
    printf 'Link already exists: %s <-> %s\n' "$id" "$target_id"
  fi
}

cmd_unlink() {
  if [ "$#" -lt 2 ]; then
    echo "Usage: $(basename "$0") unlink <id> <target-id>" >&2
    return 1
  fi

  local file target_file id target_id
  local links_a links_b new_links_a new_links_b
  local changed_a=false changed_b=false

  file=$(ticket_path "$1") || return 1
  target_file=$(ticket_path "$2") || return 1
  id=$(ticket_id_from_file "$file")
  target_id=$(ticket_id_from_file "$target_file")

  if [ "$id" = "$target_id" ]; then
    echo "Error: self link is not allowed: $id" >&2
    return 1
  fi

  links_a=$(ticket_list_field "$file" "TK_LINKS")
  list_remove_preserve_order "$links_a" "$target_id"
  new_links_a="$LIST_RESULT"
  if [ "$LIST_MUTATED" = true ]; then
    changed_a=true
  fi

  links_b=$(ticket_list_field "$target_file" "TK_LINKS")
  list_remove_preserve_order "$links_b" "$id"
  new_links_b="$LIST_RESULT"
  if [ "$LIST_MUTATED" = true ]; then
    changed_b=true
  fi

  if [ "$changed_a" = true ]; then
    upsert_property "$file" "TK_LINKS" "$new_links_a"
  fi
  if [ "$changed_b" = true ]; then
    upsert_property "$target_file" "TK_LINKS" "$new_links_b"
  fi

  if [ "$changed_a" = true ] || [ "$changed_b" = true ]; then
    printf 'Removed link: %s <-> %s\n' "$id" "$target_id"
  else
    printf 'Link not present: %s <-> %s\n' "$id" "$target_id"
  fi
}

cmd_tag() {
  if [ "$#" -lt 2 ]; then
    echo "Usage: $(basename "$0") tag <id> <tag>" >&2
    return 1
  fi

  local file id tags tag new_tags changed=false
  file=$(ticket_path "$1") || return 1
  id=$(ticket_id_from_file "$file")
  shift

  tags=$(ticket_list_field "$file" "TK_TAGS")
  for tag in "$@"; do
    list_add_unique_preserve_order "$tags" "$tag"
    new_tags="$LIST_RESULT"
    if [ "$LIST_MUTATED" = true ]; then
      changed=true
      tags="$new_tags"
    fi
  done

  if [ "$changed" = true ]; then
    upsert_property "$file" "TK_TAGS" "$tags"
    printf 'Added tag(s) to %s: %s\n' "$id" "$*"
  else
    printf 'Tag(s) already present on %s: %s\n' "$id" "$*"
  fi
}

cmd_untag() {
  if [ "$#" -lt 2 ]; then
    echo "Usage: $(basename "$0") untag <id> <tag>" >&2
    return 1
  fi

  local file id tags tag new_tags changed=false
  file=$(ticket_path "$1") || return 1
  id=$(ticket_id_from_file "$file")
  shift

  tags=$(ticket_list_field "$file" "TK_TAGS")
  for tag in "$@"; do
    list_remove_preserve_order "$tags" "$tag"
    new_tags="$LIST_RESULT"
    if [ "$LIST_MUTATED" = true ]; then
      changed=true
      tags="$new_tags"
    fi
  done

  if [ "$changed" = true ]; then
    upsert_property "$file" "TK_TAGS" "$tags"
    printf 'Removed tag(s) from %s: %s\n' "$id" "$*"
  else
    printf 'Tag(s) not present on %s: %s\n' "$id" "$*"
  fi
}

load_ticket_index() {
  declare -gA IDX_STATUS=()
  declare -gA IDX_TITLE=()
  declare -gA IDX_PRIORITY=()
  declare -gA IDX_ASSIGNEE=()
  declare -gA IDX_TAGS=()
  declare -gA IDX_DEPS=()

  local file id status title priority assignee tags deps
  while IFS= read -r file; do
    [ -n "$file" ] || continue
    id=$(ticket_id_from_file "$file")
    status=$(ticket_field "$file" "TK_STATUS")
    title=$(ticket_title "$file")
    priority=$(ticket_field "$file" "TK_PRIORITY")
    assignee=$(ticket_field "$file" "TK_ASSIGNEE")
    tags=$(ticket_field "$file" "TK_TAGS")
    deps=$(ticket_field "$file" "TK_DEPS")

    [ -n "$status" ] || status="open"
    [ -n "$title" ] || title="Untitled"
    case "$priority" in
      0|1|2|3|4) ;;
      *) priority="2" ;;
    esac
    [ -n "$tags" ] || tags="[]"
    [ -n "$deps" ] || deps="[]"

    IDX_STATUS["$id"]="$status"
    IDX_TITLE["$id"]="$title"
    IDX_PRIORITY["$id"]="$priority"
    IDX_ASSIGNEE["$id"]="$assignee"
    IDX_TAGS["$id"]="$tags"
    IDX_DEPS["$id"]="$deps"
  done < <(ticket_files_sorted)
}

cmd_ready() {
  local assignee_filter="" tag_filter=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      -a)
        [ "$#" -ge 2 ] || { echo "Error: -a requires a value" >&2; return 1; }
        assignee_filter="$2"
        shift 2
        ;;
      --assignee=*)
        assignee_filter="${1#--assignee=}"
        shift
        ;;
      -T)
        [ "$#" -ge 2 ] || { echo "Error: -T requires a value" >&2; return 1; }
        tag_filter="$2"
        shift 2
        ;;
      --tag=*)
        tag_filter="${1#--tag=}"
        shift
        ;;
      *)
        shift
        ;;
    esac
  done

  load_ticket_index

  local -a output=()
  local id status title priority assignee tags deps dep dep_status ready
  for id in "${!IDX_STATUS[@]}"; do
    status="${IDX_STATUS[$id]}"
    if [ "$status" != "open" ] && [ "$status" != "in_progress" ]; then
      continue
    fi

    assignee="${IDX_ASSIGNEE[$id]}"
    tags="${IDX_TAGS[$id]}"
    if [ -n "$assignee_filter" ] && [ "$assignee" != "$assignee_filter" ]; then
      continue
    fi
    if [ -n "$tag_filter" ] && ! list_contains "$tags" "$tag_filter"; then
      continue
    fi

    deps="${IDX_DEPS[$id]}"
    ready=true
    while IFS= read -r dep; do
      [ -n "$dep" ] || continue
      dep_status="${IDX_STATUS[$dep]:-}"
      if [ "$dep_status" != "closed" ]; then
        ready=false
        break
      fi
    done < <(list_items "$deps")

    if [ "$ready" = true ]; then
      priority="${IDX_PRIORITY[$id]}"
      title="${IDX_TITLE[$id]}"
      output+=("$(printf '%s|%s|%-8s [P%s][%s] - %s' "$priority" "$id" "$id" "$priority" "$status" "$title")")
    fi
  done

  if [ "${#output[@]}" -eq 0 ]; then
    return 0
  fi

  printf '%s\n' "${output[@]}" | sort -t '|' -k1,1n -k2,2 | cut -d '|' -f3-
}

cmd_blocked() {
  local assignee_filter="" tag_filter=""
  while [ "$#" -gt 0 ]; do
    case "$1" in
      -a)
        [ "$#" -ge 2 ] || { echo "Error: -a requires a value" >&2; return 1; }
        assignee_filter="$2"
        shift 2
        ;;
      --assignee=*)
        assignee_filter="${1#--assignee=}"
        shift
        ;;
      -T)
        [ "$#" -ge 2 ] || { echo "Error: -T requires a value" >&2; return 1; }
        tag_filter="$2"
        shift 2
        ;;
      --tag=*)
        tag_filter="${1#--tag=}"
        shift
        ;;
      *)
        shift
        ;;
    esac
  done

  load_ticket_index

  local -a output=()
  local id status title priority assignee tags deps dep dep_status blockers blockers_list
  local -a unresolved=()
  for id in "${!IDX_STATUS[@]}"; do
    status="${IDX_STATUS[$id]}"
    if [ "$status" != "open" ] && [ "$status" != "in_progress" ]; then
      continue
    fi

    assignee="${IDX_ASSIGNEE[$id]}"
    tags="${IDX_TAGS[$id]}"
    if [ -n "$assignee_filter" ] && [ "$assignee" != "$assignee_filter" ]; then
      continue
    fi
    if [ -n "$tag_filter" ] && ! list_contains "$tags" "$tag_filter"; then
      continue
    fi

    deps="${IDX_DEPS[$id]}"
    unresolved=()
    while IFS= read -r dep; do
      [ -n "$dep" ] || continue
      dep_status="${IDX_STATUS[$dep]:-}"
      if [ "$dep_status" != "closed" ]; then
        unresolved+=("$dep")
      fi
    done < <(list_items "$deps")

    if [ "${#unresolved[@]}" -eq 0 ]; then
      continue
    fi

    blockers_list=$(list_from_items "${unresolved[@]}")
    priority="${IDX_PRIORITY[$id]}"
    title="${IDX_TITLE[$id]}"
    output+=("$(printf '%s|%s|%-8s [P%s][%s] - %s <- %s' "$priority" "$id" "$id" "$priority" "$status" "$title" "$blockers_list")")
  done

  if [ "${#output[@]}" -eq 0 ]; then
    return 0
  fi

  printf '%s\n' "${output[@]}" | sort -t '|' -k1,1n -k2,2 | cut -d '|' -f3-
}

cmd_ls() {
  local status_filter="" assignee_filter="" tag_filter=""

  while [ "$#" -gt 0 ]; do
    case "$1" in
      --status=*)
        status_filter="${1#--status=}"
        shift
        ;;
      -a)
        [ "$#" -ge 2 ] || { echo "Error: -a requires a value" >&2; return 1; }
        assignee_filter="$2"
        shift 2
        ;;
      --assignee=*)
        assignee_filter="${1#--assignee=}"
        shift
        ;;
      -T)
        [ "$#" -ge 2 ] || { echo "Error: -T requires a value" >&2; return 1; }
        tag_filter="$2"
        shift 2
        ;;
      --tag=*)
        tag_filter="${1#--tag=}"
        shift
        ;;
      *)
        shift
        ;;
    esac
  done

  local file id status assignee tags deps title
  while IFS= read -r file; do
    [ -n "$file" ] || continue

    id=$(ticket_id_from_file "$file")
    status=$(ticket_field "$file" "TK_STATUS")
    assignee=$(ticket_field "$file" "TK_ASSIGNEE")
    tags=$(ticket_field "$file" "TK_TAGS")
    deps=$(ticket_field "$file" "TK_DEPS")
    title=$(ticket_title "$file")

    if [ -z "$status" ]; then
      status="open"
    fi
    if [ -z "$deps" ]; then
      deps="[]"
    fi
    if [ -z "$title" ]; then
      title="Untitled"
    fi

    if [ -n "$status_filter" ] && [ "$status" != "$status_filter" ]; then
      continue
    fi

    if [ -n "$assignee_filter" ] && [ "$assignee" != "$assignee_filter" ]; then
      continue
    fi

    if [ -n "$tag_filter" ]; then
      if ! list_contains "$tags" "$tag_filter"; then
        continue
      fi
    fi

    if [ -n "$(list_items "$deps" | head -n1 || true)" ]; then
      printf '%-8s [%s] - %s <- %s\n' "$id" "$status" "$title" "$deps"
    else
      printf '%-8s [%s] - %s\n' "$id" "$status" "$title"
    fi
  done < <(ticket_files_sorted)
}

show_output() {
  local file="$1" full="${2:-false}"

  local id status deps links created issue_type priority assignee external_ref parent tags
  id=$(ticket_id_from_file "$file")
  status=$(ticket_field "$file" "TK_STATUS")
  deps=$(ticket_field "$file" "TK_DEPS")
  links=$(ticket_field "$file" "TK_LINKS")
  created=$(ticket_field "$file" "TK_CREATED")
  issue_type=$(ticket_field "$file" "TK_TYPE")
  priority=$(ticket_field "$file" "TK_PRIORITY")
  assignee=$(ticket_field "$file" "TK_ASSIGNEE")
  external_ref=$(ticket_field "$file" "TK_EXTERNAL_REF")
  parent=$(ticket_field "$file" "TK_PARENT")
  tags=$(ticket_field "$file" "TK_TAGS")

  [ -n "$status" ] || status="open"
  [ -n "$deps" ] || deps="[]"
  [ -n "$links" ] || links="[]"
  [ -n "$issue_type" ] || issue_type="task"
  [ -n "$priority" ] || priority="2"
  [ -n "$tags" ] || tags="[]"

  printf 'id: %s\n' "$id"
  printf 'status: %s\n' "$status"
  printf 'deps: %s\n' "$deps"
  printf 'links: %s\n' "$links"
  printf 'created: %s\n' "$created"
  printf 'type: %s\n' "$issue_type"
  printf 'priority: %s\n' "$priority"
  if [ -n "$assignee" ]; then
    printf 'assignee: %s\n' "$assignee"
  fi
  if [ -n "$external_ref" ]; then
    printf 'external-ref: %s\n' "$external_ref"
  fi
  if [ -n "$parent" ]; then
    printf 'parent: %s\n' "$parent"
  fi
  printf 'tags: %s\n' "$tags"
  printf '\n'
  if [ "$full" = true ]; then
    ticket_body "$file"
  else
    ticket_outline "$file"
  fi
}

cmd_show() {
  local full=false id="" arg

  while [ "$#" -gt 0 ]; do
    arg="$1"
    case "$arg" in
      --full|-f)
        full=true
        shift
        ;;
      --)
        shift
        break
        ;;
      -*)
        echo "Usage: $(basename "$0") show [--full] <id>" >&2
        return 1
        ;;
      *)
        if [ -n "$id" ]; then
          echo "Usage: $(basename "$0") show [--full] <id>" >&2
          return 1
        fi
        id="$arg"
        shift
        ;;
    esac
  done

  if [ "$#" -gt 0 ]; then
    if [ -n "$id" ]; then
      echo "Usage: $(basename "$0") show [--full] <id>" >&2
      return 1
    fi
    id="$1"
    shift
  fi

  if [ -z "$id" ]; then
    echo "Usage: $(basename "$0") show [--full] <id>" >&2
    return 1
  fi

  local file
  file=$(ticket_path "$id") || return 1

  if [ -t 1 ] && [ -n "$TICKET_PAGER" ]; then
    read -r -a pager_cmd <<< "$TICKET_PAGER"
    show_output "$file" "$full" | "${pager_cmd[@]}"
  else
    show_output "$file" "$full"
  fi
}

find_heading_line_level() {
  local file="$1" target="$2"
  awk -v target="$target" '
  $0 ~ /^\*+[[:space:]]+/ {
    line = $0
    match(line, /^\*+/)
    level = RLENGTH
    sub(/^\*+[[:space:]]+/, "", line)
    gsub(/[[:space:]]+$/, "", line)
    if (tolower(line) == target) {
      printf "%d\t%d\n", NR, level
      exit
    }
  }
  ' "$file"
}

next_heading_at_or_above() {
  local file="$1" start_line="$2" level="$3"
  awk -v start_line="$start_line" -v level="$level" '
  NR > start_line && $0 ~ /^\*+[[:space:]]+/ {
    match($0, /^\*+/)
    current = RLENGTH
    if (current <= level) {
      print NR
      found = 1
      exit
    }
  }
  END {
    if (!found) {
      print NR + 1
    }
  }
  ' "$file"
}

first_top_level_heading() {
  local file="$1"
  awk '/^\*[[:space:]]+/ { print NR; exit }' "$file"
}

next_top_level_heading() {
  local file="$1" start_line="$2"
  awk -v start_line="$start_line" '
  NR > start_line && /^\*[[:space:]]+/ { print NR; found = 1; exit }
  END { if (!found) print NR + 1 }
  ' "$file"
}

insert_block_before_line() {
  local file="$1" insert_line="$2" block="$3"
  local tmp
  tmp=$(mktemp)

  if [ "$insert_line" -le 1 ]; then
    {
      printf '%s' "$block"
      cat "$file"
    } > "$tmp"
  else
    {
      head -n $((insert_line - 1)) "$file"
      printf '%s' "$block"
      tail -n +"$insert_line" "$file"
    } > "$tmp"
  fi

  mv "$tmp" "$file"
}

cmd_add_note() {
  if [ "$#" -lt 1 ]; then
    echo "Usage: $(basename "$0") add-note <id> [note text]" >&2
    return 1
  fi

  local file
  file=$(ticket_path "$1") || return 1
  shift

  local note
  if [ "$#" -gt 0 ]; then
    note="$*"
  elif [ ! -t 0 ]; then
    note=$(cat)
  else
    echo "Error: no note provided" >&2
    return 1
  fi

  note=$(normalize_escaped_newlines "$note")

  local ts
  ts=$(_note_timestamp)

  local note_headline note_body note_entry_heading note_entry
  note_headline=${note%%$'\n'*}
  if [ "$note_headline" = "$note" ]; then
    note_body=""
  else
    note_body=${note#*$'\n'}
  fi

  if [ -n "$note_headline" ]; then
    note_entry_heading=$(printf '*** %s %s' "$ts" "$note_headline")
  else
    note_entry_heading=$(printf '*** %s' "$ts")
  fi

  if [ -n "$note_body" ]; then
    note_entry=$(printf '\n%s\n%s\n' "$note_entry_heading" "$note_body")
  else
    note_entry=$(printf '\n%s\n' "$note_entry_heading")
  fi

  local info notes_line notes_level insert_line block
  info=$(find_heading_line_level "$file" "notes")

  if [ -n "$info" ]; then
    notes_line=$(printf '%s\n' "$info" | awk -F'\t' '{print $1}')
    notes_level=$(printf '%s\n' "$info" | awk -F'\t' '{print $2}')
    insert_line=$(next_heading_at_or_above "$file" "$notes_line" "$notes_level")
    block="$note_entry"
    insert_block_before_line "$file" "$insert_line" "$block"
  else
    local first_root next_root
    first_root=$(first_top_level_heading "$file")

    if [ -n "$first_root" ]; then
      next_root=$(next_top_level_heading "$file" "$first_root")
      block=$(printf '\n** Notes%s' "$note_entry")
      insert_block_before_line "$file" "$next_root" "$block"
    else
      printf '\n* Notes%s' "$note_entry" >> "$file"
    fi
  fi

  printf 'Note added to %s\n' "$(ticket_id_from_file "$file")"
}

ticket_to_json() {
  local file="$1"

  local id status deps links created issue_type priority assignee external_ref parent tags
  id=$(ticket_id_from_file "$file")
  status=$(ticket_field "$file" "TK_STATUS")
  deps=$(ticket_field "$file" "TK_DEPS")
  links=$(ticket_field "$file" "TK_LINKS")
  created=$(ticket_field "$file" "TK_CREATED")
  issue_type=$(ticket_field "$file" "TK_TYPE")
  priority=$(ticket_field "$file" "TK_PRIORITY")
  assignee=$(ticket_field "$file" "TK_ASSIGNEE")
  external_ref=$(ticket_field "$file" "TK_EXTERNAL_REF")
  parent=$(ticket_field "$file" "TK_PARENT")
  tags=$(ticket_field "$file" "TK_TAGS")

  [ -n "$status" ] || status="open"
  [ -n "$deps" ] || deps="[]"
  [ -n "$links" ] || links="[]"
  [ -n "$issue_type" ] || issue_type="task"
  [ -n "$priority" ] || priority="2"
  [ -n "$tags" ] || tags="[]"

  local deps_json links_json tags_json
  deps_json=$(list_to_json "$deps")
  links_json=$(list_to_json "$links")
  tags_json=$(list_to_json "$tags")

  jq -cn \
    --arg id "$id" \
    --arg status "$status" \
    --arg created "$created" \
    --arg issue_type "$issue_type" \
    --arg priority "$priority" \
    --arg assignee "$assignee" \
    --arg external_ref "$external_ref" \
    --arg parent "$parent" \
    --argjson deps "$deps_json" \
    --argjson links "$links_json" \
    --argjson tags "$tags_json" \
    '
    {
      id: $id,
      status: $status,
      deps: $deps,
      links: $links,
      created: $created,
      type: $issue_type,
      priority: $priority,
      tags: $tags
    }
    + (if $assignee != "" then {assignee: $assignee} else {} end)
    + (if $external_ref != "" then {"external-ref": $external_ref} else {} end)
    + (if $parent != "" then {parent: $parent} else {} end)
    '
}

cmd_query() {
  local filter="${1:-}"

  if ! command -v jq >/dev/null 2>&1; then
    echo "Error: jq is required for query" >&2
    return 1
  fi

  local -a objects=()
  local file obj
  while IFS= read -r file; do
    [ -n "$file" ] || continue
    obj=$(ticket_to_json "$file")
    objects+=("$obj")
  done < <(ticket_files_sorted)

  if [ "${#objects[@]}" -eq 0 ]; then
    return 0
  fi

  if [ -n "$filter" ]; then
    printf '%s\n' "${objects[@]}" | jq -c "$filter"
  else
    printf '%s\n' "${objects[@]}"
  fi
}

lint_file() {
  local file="$1"
  awk -v file="$file" '
  BEGIN {
    semantic["description"] = 1
    semantic["scope"] = 1
    semantic["design"] = 1
    semantic["acceptance criteria"] = 1
    semantic["notes"] = 1
    failed = 0
  }

  $0 ~ /^\*+[[:space:]]+/ {
    line = $0
    match(line, /^\*+/)
    level = RLENGTH

    heading = line
    sub(/^\*+[[:space:]]+/, "", heading)
    gsub(/[[:space:]]+$/, "", heading)
    lowered = tolower(heading)

    if (lowered in semantic) {
      seen[lowered]++
      if (seen[lowered] > 1) {
        printf "%s:%d: L001 duplicate semantic heading: %s\n", file, NR, heading
        failed = 1
      }
      if (level != 2) {
        printf "%s:%d: L002 semantic heading must be level-2 (**): %s\n", file, NR, heading
        failed = 1
      }
    }
  }

  END {
    exit(failed ? 1 : 0)
  }
  ' "$file"
}

cmd_lint() {
  local target="${1:-}"

  if [ "$#" -gt 1 ]; then
    echo "Usage: $(basename "$0") lint [id-or-path]" >&2
    return 2
  fi

  local -a files=()
  if [ -n "$target" ]; then
    if [ -f "$target" ]; then
      files+=("$target")
    else
      local resolved
      resolved=$(ticket_path "$target") || return 2
      files+=("$resolved")
    fi
  else
    mapfile -t files < <(ticket_files_sorted)
  fi

  if [ "${#files[@]}" -eq 0 ]; then
    return 0
  fi

  local failed=0 file
  for file in "${files[@]}"; do
    if ! lint_file "$file"; then
      failed=1
    fi
  done

  if [ "$failed" -eq 1 ]; then
    return 1
  fi

  return 0
}

cmd_help() {
  local cmd
  cmd=$(basename "$0")
  cat <<__HELP__
$cmd - minimal org-mode ticket system

Usage: $cmd <command> [args]

Commands:
  create [title] [options] Create ticket, prints ID
    -d, --description      Description text (writes ** Description)
    --scope                Scope text (writes ** Scope)
    --design               Design notes (writes ** Design)
    --acceptance           Acceptance criteria (writes ** Acceptance Criteria)
    -t, --type             Type (bug|feature|task|epic|chore) [default: task]
    -p, --priority         Priority 0-4, 0=highest [default: 2]
    -a, --assignee         Assignee
    --external-ref         External reference (e.g., gh-123, JIRA-456)
    --parent               Parent ticket ID
    --tags                 Comma-separated tags (e.g., --tags ui,backend,urgent)
  start <id>               Set status to in_progress
  block <id>               Set status to blocked (external dependency)
  close <id>               Set status to closed
  reopen <id>              Set status to open
  status <id> <status>     Update status (open|in_progress|blocked|closed)
  dep <id> <dep-id>        Add dependency (id depends on dep-id)
  undep <id> <dep-id>      Remove dependency
  link <id> <target-id>    Add symmetric link
  unlink <id> <target-id>  Remove symmetric link
  tag <id> <tag> [tag...]  Add tag(s) to ticket
  untag <id> <tag> [tag...] Remove tag(s) from ticket
  ready [-a X] [-T X]      List open/in-progress tickets with deps resolved
  blocked [-a X] [-T X]    List open/in-progress tickets with unresolved deps
  ls|list [--status=X] [-a X] [-T X] List tickets
  show [--full] <id>       Display ticket metadata and heading outline
  add-note <id> [text]     Append timestamped note (or pipe via stdin)
  query [jq-filter]        Output tickets as JSON objects, optionally filtered
  lint [id-or-path]        Validate semantic heading conventions

Tickets stored as org files in .tickets/
Supports partial ID matching (e.g., '$cmd show 5c4' matches 'nw-5c46')
__HELP__
}

case "${1:-help}" in
  help|--help|-h) ;;
  *) init_tickets_dir "${1:-}" || exit 1 ;;
esac

case "${1:-help}" in
  create) shift; cmd_create "$@" ;;
  start) shift; cmd_start "$@" ;;
  block) shift; cmd_block "$@" ;;
  close) shift; cmd_close "$@" ;;
  reopen) shift; cmd_reopen "$@" ;;
  status) shift; cmd_status "$@" ;;
  dep) shift; cmd_dep "$@" ;;
  undep) shift; cmd_undep "$@" ;;
  link) shift; cmd_link "$@" ;;
  unlink) shift; cmd_unlink "$@" ;;
  tag) shift; cmd_tag "$@" ;;
  untag) shift; cmd_untag "$@" ;;
  ready) shift; cmd_ready "$@" ;;
  blocked) shift; cmd_blocked "$@" ;;
  ls|list) shift; cmd_ls "$@" ;;
  show) shift; cmd_show "$@" ;;
  add-note) shift; cmd_add_note "$@" ;;
  query) shift; cmd_query "$@" ;;
  lint) shift; cmd_lint "$@" ;;
  help|--help|-h) cmd_help ;;
  *)
    echo "Unknown command: $1" >&2
    cmd_help >&2
    exit 1
    ;;
esac

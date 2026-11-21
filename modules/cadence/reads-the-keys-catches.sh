#!/usr/bin/env bash
# Shows reads-the-keys.sh is a guard and not only a pass: it must accept
# spend-watch's files as they are, and refuse each copy changed as a
# rename in spend-watch would change it, saying the words of the guard
# meant to catch it, so a case caught by another guard can't stand in for
# its own. Each change is made from today's file, anchored on the line it
# changes, so the copies can't drift from the source the way kept fixtures
# would, and one that changes nothing fails here rather than passing as
# caught.
# It takes reads-the-keys.sh, then the same files in the same order.
set -euo pipefail

check=$1
shift
files=("$@")
budgets=2
standing=5
status=6
config=7
work=$(mktemp -d)

if ! bash "$check" "${files[@]}" 2>"$work/said"; then
  echo "a key cadence copies is no longer the one spend-watch writes:" >&2
  cat "$work/said" >&2
  exit 1
fi

# A copy of file number $2 changed by the sed program $3 must be refused,
# with the words $4.
refused() {
  local name=$1 at=$2 copy="$work/$1.rs" given=("${files[@]}")
  sed "$3" "${files[$at]}" >"$copy"
  if cmp -s "$copy" "${files[$at]}"; then
    echo "$name changed nothing in ${files[$at]}, so it can't show the check fails" >&2
    exit 1
  fi
  given[at]=$copy
  if bash "$check" "${given[@]}" 2>"$work/said"; then
    echo "the check passed ${files[$at]} with $name" >&2
    exit 1
  fi
  if ! grep -qF -- "$4" "$work/said"; then
    echo "the check refused $name, but not by saying '$4':" >&2
    cat "$work/said" >&2
    exit 1
  fi
}

# Above the name field of the Allowance struct, rather than another's.
allowance_name="/^pub struct Allowance {\$/,/^}/{/^    pub name: String,\$/i\\"
attributes="has serde attributes"
unread="can't read whole"
gone="no longer has the line"

refused field-renamed "$standing" \
  '/^    pub nearest: Option<f64>,$/i\    #[serde(rename = "near")]' "$attributes"
refused struct-renamed "$standing" \
  '/^pub struct Standing {$/i #[serde(rename_all = "SCREAMING_SNAKE_CASE")]' "$attributes"
refused budget-renamed "$standing" \
  '/^pub struct Budget {$/i #[serde(rename_all = "UPPERCASE")]' "$attributes"
# Budget keeps its label, so only a field found in its own struct fails.
refused allowance-field-renamed "$standing" \
  '/^pub struct Allowance {$/,/^}/s/^    pub label: String,$/    pub title: String,/' \
  "Allowance $gone:     pub label: String,"
# Above a field with no attribute of its own, so only the guard on what
# it can't read whole fails it.
refused attribute-split "$standing" \
  "$allowance_name"'    #[serde(\n        rename = "id"\n    )]
}' "$unread"
refused renamed-past-a-comment "$standing" \
  "$allowance_name"'    #[serde(rename = "id")]\n    // a comment
}' "$attributes"
refused attribute-inside-another "$standing" \
  "$allowance_name"'    #[cfg_attr(test, serde(rename = "id"))]
}' "$unread"
refused version-moved "$standing" \
  's/^pub const VERSION: u32 = 1;$/pub const VERSION: u32 = 2;/' "$gone: pub const VERSION"
refused version-field-renamed "$standing" 's/^    pub v: u32,$/    pub version: u32,/' \
  "Standing $gone:     pub v: u32,"
refused folder-renamed "$config" 's/join("spend-watch")/join("spend")/' "$gone"
refused file-renamed "$status" 's/join("budgets.json")/join("standing.json")/' "$gone"
refused read-from-another-folder "$budgets" \
  's|join("spend-watch/budgets.json")|join("spend/budgets.json")|' "$gone"
refused read-at-another-version "$budgets" \
  's/^const VERSION: u32 = 1;$/const VERSION: u32 = 2;/' "$gone: const VERSION"

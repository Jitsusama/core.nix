#!/usr/bin/env bash
# Shows reads-the-keys.sh is a guard and not only a pass: it must accept
# spend-watch's files as they are, and refuse each copy changed as a
# rename in spend-watch would change it. Each change is made from today's
# file, anchored on the line it changes, so the copies can't drift from
# the source the way kept fixtures would, and one that changes nothing
# fails here rather than passing as caught.
# It takes reads-the-keys.sh, then the same files in the same order.
set -euo pipefail

check=$1
shift
files=("$@")
standing=5
config=7
work=$(mktemp -d)

if ! bash "$check" "${files[@]}" 2>"$work/said"; then
  echo "the check refused spend-watch's own files:" >&2
  cat "$work/said" >&2
  exit 1
fi

# A copy of file number $2 changed by the sed program $3 must be refused.
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
}

# Above the name field of the Allowance struct, rather than another's.
allowance_name="/^pub struct Allowance {\$/,/^}/{/^    pub name: String,\$/i\\"

refused field-renamed "$standing" \
  '/^    pub nearest: Option<f64>,$/i\    #[serde(rename = "near")]'
refused struct-renamed "$standing" \
  '/^pub struct Standing {$/i #[serde(rename_all = "SCREAMING_SNAKE_CASE")]'
refused budget-renamed "$standing" \
  '/^pub struct Budget {$/i #[serde(rename_all = "UPPERCASE")]'
refused attribute-split "$standing" \
  '/^    pub nearest: Option<f64>,$/i\    #[serde(\n        rename = "near"\n    )]'
refused renamed-past-a-comment "$standing" \
  "$allowance_name"'    #[serde(rename = "id")]\n    // a comment
}'
refused attribute-inside-another "$standing" \
  "$allowance_name"'    #[cfg_attr(test, serde(rename = "id"))]
}'
refused version-moved "$standing" \
  's/^pub const VERSION: u32 = 1;$/pub const VERSION: u32 = 2;/'
refused version-field-renamed "$standing" 's/^    pub v: u32,$/    pub version: u32,/'
refused folder-renamed "$config" 's/join("spend-watch")/join("spend")/'

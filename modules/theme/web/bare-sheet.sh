# shellcheck shell=bash
# Writes a stylesheet again without its comments, so a browser is sent only
# its rules. esbuild, unminified, writes each sheet with its rules and
# declarations in their order, where a minifier reorders and merges them;
# what else it changes reads the same, as quotes it drops round a
# selector's value. That was read sheet by sheet on esbuild 0.27, and is
# worth reading again when nixpkgs brings a printer that may do more.
#
# A sheet esbuild can't read fails, and so does one it only warns on, each
# saying which sheet and what esbuild said, rather than scrolling by unread
# in a build that passes. A comment still in the sheet once written again
# fails too: esbuild keeps one marked to stay, as a licence notice often is,
# and dropping that is a person's choice, not the build's, while the sites'
# budgets count on there being none.
# usage: bare-sheet.sh <esbuild> <sheet> <out>
#   <out> may be <sheet>, which is then written over, and names the sheet
#   in what a refusal says, since <sheet> may be a path in the store.
set -euo pipefail

esbuild=$1 sheet=$2 out=$3
name=${out##*/}
work=$(mktemp -d)
trap 'rm -rf "${work}"' EXIT

# esbuild exits 1 on a sheet it can't read; any other status is its own
# trouble, said as such, so a crash doesn't read as the sheet's fault.
status=0
"${esbuild}" --log-level=warning "${sheet}" >"${work}/bare.css" 2>"${work}/said" || status=$?
if ((status == 1)); then
  echo "${name}: esbuild couldn't read it" >&2
  cat "${work}/said" >&2
  exit 1
elif ((status != 0)); then
  echo "${name}: esbuild exited ${status} on it" >&2
  cat "${work}/said" >&2
  exit 1
fi
if [[ -s "${work}/said" ]]; then
  echo "${name}: esbuild warned on it" >&2
  cat "${work}/said" >&2
  exit 1
fi
if grep -q '/\*' "${work}/bare.css"; then
  echo "${name}: holds /* once written again, a comment esbuild kept or a string" \
    "holding one" >&2
  exit 1
fi
cp "${work}/bare.css" "${out}"

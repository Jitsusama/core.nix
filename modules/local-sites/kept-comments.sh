# shellcheck shell=bash
# Fails on a compiled script still holding a comment marked to stay. The
# TypeScript compiler drops every comment it's told to but those, as a
# licence notice often is, and dropping that is a person's choice, not the
# build's, while the sites' budgets count on there being none. Every script
# holding one is named before it fails. One script for a site's scripts
# (site.sh) and for the kit's (site.nix), so neither can skip the rule; and
# it fails when handed nothing to read, or a path that isn't a script, so a
# glob that missed can't pass for a clean build.
# usage: kept-comments.sh <script>...
set -euo pipefail

(($# > 0)) || {
  echo "no script to check" >&2
  exit 1
}
kept=0
for script in "$@"; do
  if [[ ! -f ${script} ]]; then
    echo "${script}: no such script" >&2
    kept=1
    continue
  fi
  status=0
  grep -q '/\*!' "${script}" || status=$?
  if ((status == 0)); then
    echo "${script##*/}: holds /*! once compiled, a comment marked to stay or a string" \
      "holding one" >&2
    kept=1
  elif ((status > 1)); then
    echo "${script##*/}: couldn't be read" >&2
    kept=1
  fi
done
exit "${kept}"

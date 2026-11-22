# shellcheck shell=bash
# Fails on a compiled script still holding a comment marked to stay. The
# TypeScript compiler drops every comment it's told to but those, as a
# licence notice often is, and dropping that is a person's choice, not the
# build's, while the sites' budgets count on there being none. Every script
# holding one is named before it fails. One script for a site's scripts
# (site.sh) and for the kit's (site.nix), so neither can skip the rule.
# usage: kept-comments.sh <script>...
set -euo pipefail

kept=0
for script in "$@"; do
  [[ -f ${script} ]] || continue
  if grep -q '/\*!' "${script}"; then
    echo "${script##*/}: holds /*! once compiled, a comment marked to stay or a string" \
      "holding one" >&2
    kept=1
  fi
done
exit "${kept}"

# shellcheck shell=bash
# gh as the disk guard's GitHub predicate asks it: it adds what it was asked
# to the file $ASKED names, and answers with $MERGED, the heads of merged
# pull requests one a line, as `gh pr list --jq` does. With $FAILS set, it
# fails as gh does when GitHub can't be reached.
printf '%s\n' "$*" >> "$ASKED"
if [ -n "${FAILS:-}" ]; then
  echo "error connecting to api.github.com" >&2
  exit 1
fi
printf '%s' "$MERGED"

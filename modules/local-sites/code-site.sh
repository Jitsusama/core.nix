# shellcheck shell=bash
# Writes the code site's repos.json from repos's record, whole or not at
# all: the record is read to a file beside the page under a temporary name
# and moved into place only once repos has answered in full, so the page
# never reads half of one. When repos can't read the disk, the last good
# record stays, one plain line says why, and the run still ends well: the
# page says how old its record is, and a unit failing every minute would say
# nothing more than that.
# usage: code-site.sh <site folder>
# REPOS_ROOTS names where repos looks, as for repos itself.
set -euo pipefail

site=$1
mkdir -p "$site"
work=$(mktemp -d "$site/.repos.XXXXXX")
trap 'rm -rf "$work"' EXIT

if ! repos --json > "$work/repos.json" 2> "$work/said"; then
  said=$(< "$work/said")
  said=${said//$'\n'/ }
  echo "repos couldn't read the disk (${said:0:200}); the last record stays"
  exit 0
fi

chmod 644 "$work/repos.json"
mv "$work/repos.json" "$site/repos.json"

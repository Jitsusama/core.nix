# shellcheck shell=bash
# Writes the code site's homes.json from repos's record: every repository
# and tree on the disk, each with the address Code shows it at, so any
# other site links a name it mentions to its home (the web-design skill's
# One Home for Every Thing). Code itself draws live from the repos daemon;
# these change with the disk, which Nix never builds, so they're written
# into the documents beside the page's mount, which answers with them.
#
# They're written whole or not at all: read to a file beside them under a
# temporary name and moved into place only once repos has answered in full,
# so no site reads half of them. When repos fails, as with nowhere to look,
# the last good homes stay, one plain line says why, and the run still ends
# well, since a unit failing every minute would say nothing more. A reading
# takes well under a second, but starved on a machine short of memory it can
# stall for minutes, so the writer ends one at CODE_SITE_WAITS seconds, 40
# unless said, the same way, before systemd's timeout would count the stall
# as the unit failing.
# usage: code-site.sh <site folder>
# REPOS_ROOTS names where repos looks, as for repos itself.
set -euo pipefail

site=$1
mkdir -p "$site"
work=$(mktemp -d "$site/.repos.XXXXXX")
trap 'rm -rf "$work"' EXIT

waits=${CODE_SITE_WAITS:-40}
# A wait of nothing would turn the bound off, and one past bash's arithmetic would wrap, so
# only a whole number of seconds up to four digits stands.
if [[ ! $waits =~ ^[1-9][0-9]{0,3}$ ]]; then
  waits=40
fi
# A read that won't end on the first signal is killed five seconds on, which
# timeout answers 137 for rather than 124. A read the kernel killed answers 137
# too, sooner, and is a failure rather than a stall, so the time tells them apart.
started=$SECONDS
if timeout -k 5 "$waits" repos --json > "$work/repos.json" 2> "$work/said"; then
  ended=0
else
  ended=$?
fi
if ((ended == 0)); then
  :
elif ((ended == 124 || (ended == 137 && SECONDS - started > waits))); then
  echo "repos read for over $waits s, as a machine short of memory or disk makes it;" \
    "the last homes stay"
  exit 0
else
  said=$(< "$work/said")
  said=${said//$'\n'/ }
  said=${said:0:200}
  # A read killed outright says nothing itself, so the signal says it.
  if ((ended > 128)); then
    said="killed by signal $((ended - 128))${said:+: $said}"
  fi
  echo "repos failed ($said); the last homes stay"
  exit 0
fi

# A repository is home at #repo=<owner/name>, or its name alone where it has
# no owner, and a tree at #tree=<its folder>, as Code's address says them. A
# tree whose folder is gone is no longer anywhere to link to. An empty answer
# is no record at all, which -e refuses rather than writing empty homes.
if ! jq -e '{things: (
  [.repositories[] | {
    kind: "repository",
    key,
    name,
    names: [.name],
    address: ("code/#repo=" + ((if .owner then .owner + "/" else "" end) + .name | @uri))
  }]
  + [.repositories[].checkouts[].trees[] | select(.present) | {
    kind: "tree",
    key: .path,
    name: .path,
    names: [.path],
    address: ("code/#tree=" + (.path | @uri))
  }]
)}' "$work/repos.json" > "$work/homes.json" 2> "$work/said"; then
  said=$(< "$work/said")
  said=${said//$'\n'/ }
  echo "repos's record couldn't be read (${said:0:200}); the last homes stay"
  exit 0
fi

chmod 644 "$work/homes.json"
mv "$work/homes.json" "$site/homes.json"

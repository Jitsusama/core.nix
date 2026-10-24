# shellcheck shell=bash
# Writes a store of quests into the folder given, as the quest workflow
# keeps them, for tests/quests.qml to find through pi-quests: a driving
# quest with a subquest under it, one blocked, one concluded, a folder with
# no README, one whose README has no front matter, and one written later,
# in later/, for the launcher to see arrive. The folders the quests' trees
# and sessions name are made beside it, but for one of each that's gone.
# usage: quests-store.sh <folder>
set -euo pipefail

here=$1
cd "$here"
mkdir -p store/QEST-20261005-NOTES store/QEST-20261006-PLAIN older newer session \
  later/QEST-20261020-NEWONE

# Writes a quest's README: its front matter from the input, then its title,
# then a heading that isn't its title.
quest() {
  mkdir -p "store/$1"
  {
    echo ---
    echo "id: $1"
    cat
    echo ---
    echo
    echo "# $2"
    echo
    echo "# Not the title"
  } > "store/$1/README.md"
}

quest QEST-20261001-LAPTOP "Bootstrap the laptop" << EOF
kind: quest
parent: null
status: active
priority: driving
rank: 1
updated: 2026-10-22
trees:
  - path: $here/older
  - path: $here/newer
    providerId: dev-tree
EOF
quest QEST-20261003-PROMPT "Retune the prompt" << EOF
kind: subquest
parent: QEST-20261001-LAPTOP
status: active
priority: queued
rank: 2
updated: 2026-10-21
sessions:
  - id: one
    cwd: $here/session
    started: 2026-10-03T10:00:00.000Z
  - id: two
    cwd: $here/gone
    started: 2026-10-04T10:00:00.000Z
trees:
  - path: $here/missing
EOF
quest QEST-20261004-STICK "Wait on the stick" << EOF
kind: quest
status: blocked
priority: bench
rank: 1
updated: 2026-10-04
EOF
quest QEST-20250105-BANNER "Tune the banner" << EOF
kind: quest
status: concluded
priority: someday
updated: 2025-01-05
EOF
echo "# No front matter" > store/QEST-20261006-PLAIN/README.md
quest QEST-20261020-NEWONE "Newly made" << EOF
kind: quest
status: active
priority: someday
updated: 2026-10-20
EOF
mv store/QEST-20261020-NEWONE/README.md later/QEST-20261020-NEWONE/
rmdir store/QEST-20261020-NEWONE

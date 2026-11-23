# shellcheck shell=bash # The check runs it with bash.
# Runs chunks-given-back.sh, the script named first, with a stand-in for
# btrfs. Its usage says the disk's size and the bytes in no chunk, kept in
# a file, and each balance step adds what a file of steps gives that step,
# or fails for a step named in it as failing. It balances only while short,
# stops at the first step that frees enough, goes past a step that fails,
# and stops at half-full chunks.
set -euo pipefail

given=$1
work=$(mktemp -d)
mkdir "$work/bin"
gib=$((1024 * 1024 * 1024))

cat >"$work/bin/btrfs" <<EOF
#!$BASH
set -euo pipefail
if [ "\$1" = filesystem ]; then
  printf 'Overall:\n    Device size:\t\t%s\n    Device unallocated:\t\t%s\n' \
    "\$(cat "$work/size")" "\$(cat "$work/free")"
  exit 0
fi
step=\${3#-dusage=}
echo "\$step" >>"$work/steps"
gives=\$(awk -v step="\$step" '\$1 == step { print \$2 }' "$work/gives")
if [ "\$gives" = fails ]; then
  exit 1
fi
echo \$((\$(cat "$work/free") + \${gives:-0})) >"$work/free"
EOF
chmod +x "$work/bin/btrfs"

# A disk of SIZE GiB with FREE GiB in no chunk, whose steps give what
# each line of the rest says: a step's fullness, then GiB or "fails".
disk() {
  echo $(($1 * gib)) >"$work/size"
  echo $(($2 * gib)) >"$work/free"
  shift 2
  : >"$work/steps"
  : >"$work/gives"
  for line in "$@"; do
    read -r step gives <<<"$line"
    if [ "$gives" = fails ]; then
      echo "$step fails" >>"$work/gives"
    else
      echo "$step $((gives * gib))" >>"$work/gives"
    fi
  done
}

give() {
  PATH="$work/bin:$PATH" bash "$given" / >/dev/null
}

expect() {
  local steps
  steps=$(tr '\n' ' ' <"$work/steps")
  if [ "$steps" != "$1" ]; then
    echo "$2, but the steps run were '$steps', not '$1'"
    exit 1
  fi
}

disk 950 60
give
expect "" "a disk with 50 GiB in no chunk is left alone"

disk 200 20
give
expect "" "a small disk with a tenth of it in no chunk is left alone"

disk 950 0 "0 5" "5 10" "10 40" "20 40"
give
expect "0 5 10 " "balancing stops at the first step that frees enough"

disk 950 0 "0 fails" "5 fails" "10 60"
give
expect "0 5 10 " "a step that fails is passed for the next"

disk 950 0 "0 1"
give
expect "0 5 10 20 30 40 50 " "balancing stops at half-full chunks"

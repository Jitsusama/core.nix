# shellcheck shell=bash # The check runs it with bash.
# Runs chunks-given-back.sh, the script named first, with a stand-in for
# btrfs that answers only the two calls the script may make, on /. Its
# usage says the disk's size and the bytes in no chunk, kept in files,
# and each balance step adds what a file of steps gives that step, or
# fails for a step named in it as failing. It balances only by night and
# while short, stops at the first step that frees enough, goes past a
# step that fails, stops at half-full chunks, and fails when it ends
# short or can't read the usage.
set -euo pipefail

given=$1
work=$(mktemp -d)
mkdir "$work/bin"
gib=$((1024 * 1024 * 1024))

cat >"$work/bin/btrfs" <<EOF
#!$BASH
set -euo pipefail
if [ "\$*" = "filesystem usage -b /" ]; then
  if [ -e "$work/garbled" ]; then
    printf 'Overall:\n    Device size:\t\t952.85GiB\n'
    exit 0
  fi
  printf 'Overall:\n    Device size:\t\t%s\n    Device unallocated:\t\t%s\n' \
    "\$(cat "$work/size")" "\$(cat "$work/free")"
  exit 0
fi
if [ "\$#" -ne 4 ] || [ "\$1 \$2" != "balance start" ] || [ "\$4" != / ] \
  || ! [[ \$3 =~ ^-dusage=[0-9]+$ ]]; then
  echo "the stand-in for btrfs was asked: \$*" >&2
  exit 99
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
  rm -f "$work/garbled"
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

# Runs the script at HOUR (03 unless given), and says how it ended.
give() {
  local status=0
  HOUR=${HOUR:-03} PATH="$work/bin:$PATH" bash "$given" / >/dev/null 2>"$work/err" || status=$?
  echo "$status" >"$work/status"
}

expect() {
  local steps status
  steps=$(tr '\n' ' ' <"$work/steps")
  status=$(cat "$work/status")
  if [ "$steps" != "$1" ] || [ "$status" != "$2" ]; then
    echo "$3, but the steps run were '$steps' and it ended $status, not '$1' and $2"
    cat "$work/err"
    exit 1
  fi
}

disk 950 50
give
expect "" 0 "a disk with 50 GiB in no chunk is left alone"

disk 200 20
give
expect "" 0 "a small disk with a tenth of it in no chunk is left alone"

disk 950 0 "0 5" "5 10" "10 40" "20 40"
give
expect "0 5 10 " 0 "balancing stops at the first step that frees enough"

disk 950 0 "0 50"
give
expect "0 " 0 "a step that frees exactly enough is the last"

disk 950 0 "0 fails" "5 fails" "10 60"
give
expect "0 5 10 " 0 "a step that fails is passed for the next"

disk 950 0 "0 1"
give
expect "0 5 10 20 30 40 50 " 1 "balancing stops at half-full chunks and fails short"

disk 950 0 "0 fails" "5 fails" "10 fails" "20 fails" "30 fails" "40 fails" "50 fails"
give
expect "0 5 10 20 30 40 50 " 1 "a night when every step fails fails"

disk 950 0
touch "$work/garbled"
give
expect "" 2 "usage it can't read balances nothing and fails"

disk 950 0 "0 50"
HOUR=09 give
expect "" 0 "a start by day balances nothing"

disk 950 0 "0 50"
HOUR=00 give
expect "" 0 "a start before 01:00 balances nothing"

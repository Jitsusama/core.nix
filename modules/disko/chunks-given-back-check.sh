# shellcheck shell=bash # The check runs it with bash.
# Runs chunks-given-back.sh, the script named first, with stand-ins for
# btrfs, date and timeout, each answering only the calls the script may
# make. btrfs's usage says the disk's size and the bytes in no chunk,
# kept in files, and each balance step adds what a file of steps gives
# that step, fails, runs until timeout ends it at 06:00, outlives that by
# a minute and is killed, or is killed by day. date reads a clock a step
# can move, on a day whose 06:00 a file can bring forward as a night the
# clocks go forward does, and timeout notes each step's deadline. It
# balances only from 01:00 to 06:00 and while short, ends a step at
# 06:00, stops at the first step that frees enough, goes past a step that
# fails or is killed by day, stops at half-full chunks, and fails when it
# ends short or can't read the usage.
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
# A step the file doesn't name gives nothing.
read -r gives after < <(awk -v step="\$step" '\$1 == step { print \$2, \$3 }' "$work/gives") || true
if [ -n "\${after:-}" ]; then
  echo "\$after" >"$work/clock"
fi
case "\${gives:-0}" in
fails) exit 1 ;;
ends)
  echo 06:00:00 >"$work/clock"
  exit 124
  ;;
ends+*)
  echo \$((\$(cat "$work/free") + \${gives#ends+})) >"$work/free"
  echo 06:00:00 >"$work/clock"
  exit 124
  ;;
killed)
  echo 06:01:00 >"$work/clock"
  exit 137
  ;;
killed-by-day) exit 137 ;;
*) echo \$((\$(cat "$work/free") + \${gives:-0})) >"$work/free" ;;
esac
EOF

# The day starts at a round epoch, and 06:00 falls the seconds in the
# shift file after the hour the clock face says. The clock face's epoch
# is taken as the hours before a change of clocks, which is all a case
# starting at 01:30 needs.
cat >"$work/bin/date" <<EOF
#!$BASH
day=864000000
case "\$*" in
"+%H:%M:%S %s")
  IFS=: read -r h m s <"$work/clock"
  echo "\$h:\$m:\$s \$((day + 10#\$h * 3600 + 10#\$m * 60 + 10#\$s))"
  ;;
"-d 06:00 +%s") echo \$((day + 21600 + \$(cat "$work/shift"))) ;;
*)
  echo "the stand-in for date was asked: \$*" >&2
  exit 99
  ;;
esac
EOF

cat >"$work/bin/timeout" <<EOF
#!$BASH
if [ "\$1" != --kill-after=60 ] || ! [[ \$2 =~ ^[0-9]+$ ]] || [ "\$3" != btrfs ]; then
  echo "the stand-in for timeout was asked: \$*" >&2
  exit 99
fi
echo "\$2" >>"$work/deadlines"
shift 2
exec "\$@"
EOF
chmod +x "$work/bin/btrfs" "$work/bin/date" "$work/bin/timeout"

# A disk of SIZE GiB with FREE GiB in no chunk, whose steps give what
# each line of the rest says: a step's fullness, then GiB, "fails",
# "ends", "killed" or "killed-by-day", and the clock after it, if the
# step moves it.
disk() {
  rm -f "$work/garbled"
  echo $(($1 * gib)) >"$work/size"
  echo $(($2 * gib)) >"$work/free"
  shift 2
  : >"$work/steps"
  : >"$work/gives"
  : >"$work/deadlines"
  for line in "$@"; do
    read -r step gives after <<<"$line"
    case "$gives" in
    fails | ends | killed | killed-by-day) echo "$step $gives $after" >>"$work/gives" ;;
    ends+*) echo "$step ends+$((${gives#ends+} * gib)) $after" >>"$work/gives" ;;
    *) echo "$step $((gives * gib)) $after" >>"$work/gives" ;;
    esac
  done
}

# Runs the script at CLOCK (03:00:00 unless given), on a night whose
# 06:00 is SHIFT seconds off the clock face's, and notes how it ended.
give() {
  local status=0
  echo "${CLOCK:-03:00:00}" >"$work/clock"
  echo "${SHIFT:-0}" >"$work/shift"
  PATH="$work/bin:$PATH" bash "$given" / >"$work/out" 2>"$work/err" || status=$?
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

expect_said() {
  if ! grep -qF "$1" "$work/out" "$work/err"; then
    echo "$2, but it didn't say '$1'"
    cat "$work/out" "$work/err"
    exit 1
  fi
}

expect_deadlines() {
  local deadlines
  deadlines=$(tr '\n' ' ' <"$work/deadlines")
  if [ "$deadlines" != "$1" ]; then
    echo "$2, but the steps' deadlines were '$deadlines', not '$1'"
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
expect_deadlines "10800 10800 10800 " "each step at 03:00 is given until 06:00"

disk 950 0 "0 50"
give
expect "0 " 0 "a step that frees exactly enough is the last"

disk 950 0 "0 fails" "5 fails" "10 60"
give
expect "0 5 10 " 0 "a step that fails is passed for the next"

disk 950 0 "0 1"
give
expect "0 5 10 20 30 40 50 " 1 "balancing stops at half-full chunks and fails short"
expect_said "after chunks up to half full" "a night whose steps gave too little says files must go"

disk 950 0 "0 fails" "5 fails" "10 fails" "20 fails" "30 fails" "40 fails" "50 fails"
give
expect "0 5 10 20 30 40 50 " 1 "a night when every step fails fails"
expect_said "no balance step finished" "a night when every step fails says none finished"

disk 950 0
touch "$work/garbled"
give
expect "" 65 "usage it can't read balances nothing and fails"

disk 950 0 "0 50"
CLOCK=09:00:00 give
expect "" 0 "a start by day balances nothing"

disk 950 0 "0 50"
CLOCK=00:59:59 give
expect "" 0 "a start before 01:00 balances nothing"

disk 950 0 "0 50"
CLOCK=06:00:00 give
expect "" 0 "a start at 06:00 balances nothing"

disk 950 0 "0 50"
CLOCK=01:00:00 give
expect "0 " 0 "a start at 01:00 balances"

disk 950 0 "0 50"
CLOCK=05:59:00 give
expect "0 " 0 "a step begun a minute before 06:00 runs"
expect_deadlines "60 " "a step begun a minute before 06:00 is given that minute"

disk 950 0 "0 1 06:00:00" "5 50"
CLOCK=05:00:00 give
expect "0 " 1 "a night that reaches 06:00 between steps stops and fails short"

disk 950 0 "0 1 05:00:00" "5 50"
give
expect "0 5 " 0 "a step after the clock moves is run"
expect_deadlines "10800 3600 " "each step is given only until 06:00, read again before it"

disk 950 0 "0 ends" "5 50"
CLOCK=05:30:00 give
expect "0 " 1 "a step ended at 06:00 is the last, and the night fails short"
expect_said "was ended at 06:00" "a step ended at 06:00 says so"
expect_said "the rest waits for the next night" "a night ended at 06:00 says when the rest runs"

disk 950 0 "0 killed" "5 50"
CLOCK=05:30:00 give
expect "0 " 1 "a step killed a minute past 06:00 is the last, and the night fails short"
expect_said "was killed after 06:00" "a step killed past 06:00 says so"

disk 950 0 "0 killed-by-day" "5 50"
give
expect "0 5 " 0 "a step killed before 06:00 is passed for the next"
expect_said "was killed, so the next step is tried" "a step killed before 06:00 says so"

disk 950 0 "0 1" "5 1" "10 1" "20 1" "30 1" "40 1" "50 ends"
CLOCK=05:30:00 give
expect "0 5 10 20 30 40 50 " 1 "a half-full step ended at 06:00 fails short"
expect_said "the rest waits for the next night" \
  "a half-full step ended at 06:00 says the rest waits"
if grep -qF "needs files gone" "$work/out" "$work/err"; then
  echo "a half-full step ended at 06:00 said files must go, though the night was cut short"
  exit 1
fi

disk 950 0 "0 ends+50" "5 50"
CLOCK=05:30:00 give
expect "0 " 0 "a step ended at 06:00 that freed enough leaves enough"
expect_said "GiB is in no chunk now" "a step ended at 06:00 that freed enough says so"

disk 950 0 "0 50"
CLOCK=01:30:00 SHIFT=-3600 give
expect_deadlines "12600 " "a night the clocks go forward gives a step only the time until 06:00"

disk 950 0 "0 50"
CLOCK=01:30:00 SHIFT=3600 give
expect_deadlines "19800 " "a night the clocks go back gives a step the hour it gains"

disk 950 0 "0 1" "5 1" "10 1" "20 1" "30 1" "40 1" "50 killed-by-day"
give
expect "0 5 10 20 30 40 50 " 1 "a night whose half-full step is killed by day fails short"
expect_said "up to half full didn't finish" "a half-full step killed by day says it didn't finish"
if grep -qF "needs files gone" "$work/out" "$work/err"; then
  echo "a half-full step killed by day said files must go, though it didn't finish"
  exit 1
fi

disk 950 0 "0 1" "5 1" "10 1" "20 1" "30 1" "40 1" "50 fails"
give
expect "0 5 10 20 30 40 50 " 1 "a night whose half-full step fails fails short"
expect_said "needs files gone" "a half-full step failing, as for want of room, says files must go"

disk 950 0 "0 1" "5 1" "10 1" "20 1" "30 1" "40 1" "50 killed"
CLOCK=05:30:00 give
expect "0 5 10 20 30 40 50 " 1 "a half-full step killed past 06:00 fails short"
expect_said "the rest waits for the next night" \
  "a half-full step killed past 06:00 says the rest waits"
if grep -qF "needs files gone" "$work/out" "$work/err"; then
  echo "a half-full step killed past 06:00 said files must go, though the night was cut short"
  exit 1
fi

# shellcheck shell=bash
# Holds what runs-every-case.sh says of each way a case can end, over
# stand-in cases a stand-in qml plays out, under a limit of a second of the
# processor's: a pass, a failure, a case past its own time and one killed
# from outside with a pass after it, under a guard of a minute by the clock, so a loaded builder
# slowing the spinning case never makes it a hang; a second bash's `time`
# rounded up to and wrote as 0.:00, read as that second; then, under a guard of
# three seconds, a hang that stops the run and one that ignores the guard's
# signal until its kill, each with a case left unrun after it; limits
# refused as a case is run, past the runner's first try, leaving it unrun; a
# builder holding the run below a case's hard limit, every case left unrun
# with the limit in force said once, wherever this user can't raise a hard
# limit; limits and guards that aren't whole seconds, refused; and a second
# scratch file that can't be made, every case left unrun and the first let go.
# A run whose cases fail lets both go as it ends, each seen made there first.
# usage: runs-every-case-test.sh <runs-every-case.sh>
# CAPPED_MUST_RUN, set by the check, fails the run where the capped case
# can't be shown, rather than leaving it out, so a build passes only having
# shown it.
set -euo pipefail

runner=$1
here=$(mktemp -d)
trap 'rm -rf "$here"' EXIT

# The stand-in qml plays out what its case's one line says, in the shell
# running this, as a build has no /usr/bin/env.
echo "#!$BASH" >"$here/qml"
cat >>"$here/qml" <<'EOF'
read -r how <"$1"
case $how in
  pass) exit 0 ;;
  # A case that leaves a mark that it ran, for a case that mustn't.
  mark) : >"$1.ran" ;;
  fail) exit 2 ;;
  spin) while :; do :; done ;;
  die) kill -KILL $$ ;;
  # A hang is a case that waits on nothing past the runner's wall limit, using no CPU.
  hang) exec sleep 60 ;;
  deaf)
    trap '' TERM
    # A deaf hang ignores the guard's signal and outlasts its kill, so only the kill ends it.
    exec sleep 120
    ;;
esac
EOF
chmod +x "$here/qml"

# A ulimit refusing a case's hard limit only as run() sets it, which no
# builder does once the runner's first try passed, so the case that must
# still never run can be shown. Bash reads it from BASH_ENV as it starts,
# and a function is found before the builtin of its name.
cat >"$here/refusing" <<'EOF'
ulimit() {
  if [ "$1" = -Ht ] && [[ " ${FUNCNAME[*]} " == *" run "* ]]; then
    echo "ulimit: refused as the case is run" >&2
    return 1
  fi
  builtin ulimit "$@"
}
EOF

# A mktemp making the runner's first scratch file and refusing its second,
# read from BASH_ENV as the refusing ulimit is. Each is made in a subshell,
# so the first leaves a mark in the scratch folder for the second to see,
# naming the file it made, so a case can tell one let go from one never made.
# mktemp's own failure is passed on, whatever the runner's shell options.
cat >"$here/making-one" <<'EOF'
mktemp() {
  local made
  if [ -e "$TMPDIR/.made" ]; then
    echo "mktemp: refused the second" >&2
    return 1
  fi
  made=$(command mktemp "$@") || return
  echo "$made" >"$TMPDIR/.made"
  echo "$made"
}
EOF

# A mktemp naming each file it makes in a list outside the scratch folder,
# so a run that leaves that folder empty is seen to have made its files
# there rather than nowhere.
cat >"$here/recording" <<EOF
mktemp() {
  local made
  made=\$(command mktemp "\$@") || return
  echo "\$made" >>"$here/made-ends"
  echo "\$made"
}
EOF

# The timing bash's `time` writes of a case whose second it rounded up to
# but never carried, its seconds and the clock's each 1.000, read from
# BASH_ENV as the refusing ulimit is: the runner reads a case's timing as
# the last line `tail` gives of it, and a function is found before the
# program of its name.
cat >"$here/rounding" <<'EOF'
tail() {
  echo "0.:00 0.000 0.:00"
}
EOF

fails=0
failing() {
  echo "FAIL: $1" >&2
  fails=$((fails + 1))
}

# Runs the runner over a folder of its own, with a stand-in case for each
# `name:how` given, under `limit` seconds of the processor's and `guard` of
# the clock's, keeping what it said and how it ended; where `cap` is set,
# under a hard limit of that many seconds already, which a user not root
# can't raise, and a soft one two below it, so a hold said by the soft one
# shows; where `start` is set, with bash first reading that file; and where
# `scratch` is set, with its scratch files made there.
cap=
start=
scratch=
runs() {
  local folder=$here/$1 limit=$2 guard=$3 each
  shift 3
  mkdir "$folder"
  for each in "$@"; do
    echo "${each#*:}" >"$folder/${each%%:*}.qml"
  done
  code=0
  # TMPDIR is set only where scratch is, so a caller without one keeps none.
  said=$(cd "$folder" && { [ -z "$cap" ] || { ulimit -St $((cap - 2)) && ulimit -Ht "$cap"; }; } &&
    { [ -z "$scratch" ] || export TMPDIR=$scratch; } &&
    BASH_ENV=$start CASE_LIMIT=$limit CASE_GUARD=$guard \
    "$BASH" "$runner" "$here/qml" 2>&1) ||
    code=$?
  echo "$said"
}
ends() {
  [ "$code" = "$1" ] || failing "it ended $code, not $1"
}
holds() {
  grep -qxF -- "$1" <<<"$said" || failing "it never said: $1"
}
never() {
  if grep -qF -- "$1" <<<"$said"; then
    failing "it said: $1"
  fi
}
# That a case said its time, its own seconds and the clock's each matching
# a pattern.
took() {
  grep -qE "^$1\.qml took ($2) s of its 1, ($3) s by the clock$" <<<"$said" ||
    failing "$1.qml never said its time as $2 of its own and $3 by the clock"
}

# Its two scratch files are made in a folder of its own, which it leaves
# empty.
scratch=$here/scratch-ends
mkdir "$scratch"
start=$here/recording
runs ends 1 60 a-passes:pass b-fails:fail c-spins:spin d-dies:die e-after:pass
start=
scratch=
ends 1
made=()
[ ! -e "$here/made-ends" ] || mapfile -t made <"$here/made-ends"
[ "${#made[@]}" = 2 ] || failing "the run made ${#made[@]} scratch files, not 2: ${made[*]}"
for each in "${made[@]}"; do
  [[ $each == "$here/scratch-ends/"?* ]] || failing "a scratch file was made elsewhere: $each"
done
left=$(ls -A "$here/scratch-ends")
[ -z "$left" ] || failing "the run left its scratch folder holding: $left"
holds "b-fails.qml failed, exit code 2"
holds "c-spins.qml ran past its 1 s of its own time"
holds "d-dies.qml was killed: out of memory, or past its hard 11 s"
holds "3 of 5 cases failed:"
never "a-passes.qml failed"
never "e-after.qml failed"
never "left unrun"
took a-passes 0 "[0-9]+"
took b-fails 0 "[0-9]+"
# The spinning case's own second is all the processor's, however slow the
# clock ran.
took c-spins "1|2" "[0-9]+"
# The killed one says its time too, though bash writes its job notice above
# its timing.
took d-dies 0 "[0-9]+"
# A kill from outside is no hang, so the run goes on past it.
took e-after 0 "[0-9]+"

# A second bash rounded up to and wrote as 0.:00 is read as the whole
# second it is, its own and the clock's, rather than as time unread.
start=$here/rounding
runs rounded-up 1 60 a-passes:pass
start=
ends 0
never "time couldn't be read"
took a-passes 1 1

runs hangs 1 3 e-hangs:hang f-follows:pass
ends 1
holds "e-hangs.qml ran past the clock's 3 s, waiting on something"
holds "left unrun after e-hangs.qml: f-follows.qml"
holds "1 of 2 cases failed:"
holds "  f-follows.qml left unrun"
never "f-follows.qml took"
took e-hangs 0 "[3-9]|[1-9][0-9]+"

# The guard kills it ten seconds after the signal it ignored, and a kill
# that late by the clock is still the guard's, so the run stops there too.
runs deaf 1 3 g-ignores:deaf h-follows:pass
ends 1
holds "g-ignores.qml ran past the clock's 3 s, waiting on something"
holds "left unrun after g-ignores.qml: h-follows.qml"
never "g-ignores.qml was killed"
# Its stand-in would end by itself at 120 s, so a run reaching that has
# lost the guard's kill.
took g-ignores 0 "1[3-9]|[2-5][0-9]"

# Refused as each case is run, a case fails unrun by 125 rather than running
# with no limit, and takes no time worth saying.
start=$here/refusing
runs refused-in-run 1 60 a-marks:mark b-marks:mark
start=
ends 1
holds "a-marks.qml never ran: its limits couldn't be set, or timeout itself failed (125)"
holds "b-marks.qml never ran: its limits couldn't be set, or timeout itself failed (125)"
holds "2 of 2 cases failed:"
never "a-marks.qml took"
[ ! -e "$here/refused-in-run/a-marks.qml.ran" ] || failing "a-marks.qml ran though refused"
[ ! -e "$here/refused-in-run/b-marks.qml.ran" ] || failing "b-marks.qml ran though refused"

# Without its second scratch file the runner stops before any case, and
# lets go the first, so only the stand-in's mark is left.
scratch=$here/scratch
mkdir "$scratch"
start=$here/making-one
runs one-scratch-file 1 60 a-marks:mark
start=
scratch=
ends 1
holds "the runner couldn't make its scratch files"
# mktemp's own reason reaches the log, as the runner passes it on.
holds "mktemp: refused the second"
never "a-marks.qml"
[ ! -e "$here/one-scratch-file/a-marks.qml.ran" ] || failing "a-marks.qml ran"
# A mark never made is said as a failure, rather than ending this run here.
first=
[ ! -e "$here/scratch/.made" ] || first=$(<"$here/scratch/.made")
[[ $first == "$here/scratch/"?* ]] || failing "the stand-in made no first scratch file: '$first'"
left=$(ls -A "$here/scratch")
[ "$left" = .made ] || failing "the runner left its scratch folder holding: $left"

runs refused soon 60 a-passes:pass
ends 2
holds "CASE_LIMIT and CASE_GUARD must be whole seconds, not 'soon' and '60'"
never "a-passes.qml took"

runs refused-guard 1 0 a-passes:pass
ends 2
holds "CASE_LIMIT and CASE_GUARD must be whole seconds, not '1' and '0'"
never "a-passes.qml took"

runs refused-zero 0 60 a-passes:pass
ends 2
holds "CASE_LIMIT and CASE_GUARD must be whole seconds, not '0' and '60'"
never "a-passes.qml took"

# Its hard limit of 16 s is past the 5 s already in force, so it's refused,
# once for the run, and no case runs rather than running with no limit. Only
# a user who can't raise a hard limit is refused, so where this one can, as
# root can, the case can't be shown: outside the check it says so, and in
# it that fails.
if (ulimit -t 5 && ulimit -Ht 11) 2>/dev/null; then
  if [ -n "${CAPPED_MUST_RUN:-}" ]; then
    failing "the capped case can't be shown: this user may raise a hard limit"
  else
    echo "left out the capped case: this user may raise a hard limit"
  fi
else
  cap=5
  runs capped 6 60 a-marks:mark b-marks:mark
  cap=
  ends 1
  hold="the builder holds each case to 5 s of its own time, short of the 16 s each needs,"
  # ulimit's own reason follows, whatever its words, so some must.
  grep -qE -- "$hold so no CASE_LIMIT fits; ulimit said: [^ ]" <<<"$said" ||
    failing "it never said the builder's hold once, with its figure, what fits and why"
  # grep counts none by failing, which the count already says.
  told=$(grep -c 'the builder holds' <<<"$said" || true)
  [ "$told" = 1 ] || failing "it said the hold $told times, not once"
  holds "left every case unrun"
  never "a-marks.qml"
  [ ! -e "$here/capped/a-marks.qml.ran" ] || failing "a-marks.qml ran"
  [ ! -e "$here/capped/b-marks.qml.ran" ] || failing "b-marks.qml ran"
  # A hold that leaves room names the largest limit that fits.
  cap=40
  runs capped-room 35 60 a-marks:mark
  cap=
  ends 1
  room="short of the 45 s each needs, so CASE_LIMIT must be at most 30; ulimit said: "
  grep -qF -- "$room" <<<"$said" ||
    failing "it never named the largest limit that fits"
  [ ! -e "$here/capped-room/a-marks.qml.ran" ] || failing "a-marks.qml ran under room"
  # A case needs ten seconds past its limit, so a hold of 11 leaves room for
  # a limit of 1, and one of 10 for none.
  cap=11
  runs capped-11 6 60 a-marks:mark
  cap=
  ends 1
  grep -qF -- "to 11 s of its own time, short of the 16 s each needs, so CASE_LIMIT must be at" \
    <<<"$said" || failing "a hold of 11 s wasn't said by its hard limit"
  grep -qF -- "so CASE_LIMIT must be at most 1;" <<<"$said" ||
    failing "a hold of 11 s didn't leave room for a limit of 1"
  cap=10
  runs capped-10 6 60 a-marks:mark
  cap=
  ends 1
  grep -qF -- "to 10 s of its own time, short of the 16 s each needs, so no CASE_LIMIT fits;" \
    <<<"$said" || failing "a hold of 10 s left room for a limit"
fi

[ "$fails" = 0 ]

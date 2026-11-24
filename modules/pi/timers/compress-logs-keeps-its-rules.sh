# shellcheck shell=bash
# Runs the nightly log rewrite's own script, the file it's given, under
# bash with the options writeShellApplication gives it, over a folder of
# logs in a build's sandbox, with stand-ins for what needs a real btrfs:
# a stat that says the folder is on btrfs, a filefrag that reports each
# log's extents as compressed or not, and a btrfs that rewrites a log as
# each case asks. It passes when the rewrite keeps each of its rules:
# a young log and a small one never reach btrfs; one already compressed
# enough, or reopened after the walk found it, is left alone; a failed rewrite ends 1, saying so, with the log
# as it was; a rewrite that leaves a byte different ends 1; a log written
# to during its rewrite is passed over, failing nothing; and a rewrite
# that keeps the bytes ends 0, counting it. The built program can't be
# run this way, since its own tools come first on its PATH.
set -u
script=$1
real_stat=$(command -v stat)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
tools=$work/tools
mkdir "$tools"

# A build's sandbox has no /usr/bin/env, so each stand-in names this bash.
cat > "$tools/stat" << EOF
#!$BASH
if [ "\$1" = -f ]; then
  echo btrfs
  exit 0
fi
exec $real_stat "\$@"
EOF

# filefrag -v's line for one extent, its length fourth when split on
# colons, with "encoded" among its flags where FAKE_ENCODED says so. Where
# FAKE_REOPEN says so it first writes to the log, as a session reopened
# after the walk found it would.
echo "#!$BASH" > "$tools/filefrag"
cat >> "$tools/filefrag" << 'EOF'
[ "${FAKE_REOPEN:-no}" = yes ] && echo '{"type":"message"}' >> "${*: -1}"
flags=last,eof
[ "${FAKE_ENCODED:-no}" = yes ] && flags=last,encoded,eof
echo "Filesystem type is: 9123683e"
echo " ext:     logical_offset:        physical_offset: length:   expected: flags:"
echo "   0:        0..      31:     123..       154:     32:             $flags"
EOF

# Each call is noted, then the log is rewritten as FAKE_BTRFS says:
# failing, flipping a byte with its length and time kept, appending as a
# session would, or keeping it as it was.
echo "#!$BASH" > "$tools/btrfs"
cat >> "$tools/btrfs" << 'EOF'
log=${*: -1}
echo "$log" >> "$FAKE_CALLS"
case ${FAKE_BTRFS:-same} in
fail) exit 1 ;;
flip)
  cp -p "$log" "$log.was"
  printf 'X' | dd of="$log" bs=1 seek=0 conv=notrunc status=none
  touch -r "$log.was" "$log"
  rm "$log.was"
  ;;
append) echo '{"type":"message"}' >> "$log" ;;
*) ;;
esac
EOF
chmod +x "$tools/stat" "$tools/filefrag" "$tools/btrfs"

# Nothing waits on the disk here.
quiet=$work/pressure
echo "full avg10=0.00 avg60=0.00 avg300=0.00 total=0" > "$quiet"

failures=0
fail() {
  echo "$*" >&2
  failures=$((failures + 1))
}

# A fresh sessions folder holding one old log past 64 KiB, named $1.
old_log() {
  local sessions=$work/$1
  mkdir -p "$sessions/project"
  head -c 100000 /dev/zero | tr '\0' 'a' > "$sessions/project/$1.jsonl"
  touch -d '5 days ago' "$sessions/project/$1.jsonl"
  echo "$sessions"
}

# Runs the rewrite over the folder $1, keeping what it says in $said and
# how it ended in $status; the calls btrfs had are in $work/calls. The
# stand-ins' FAKE_ENCODED, FAKE_REOPEN and FAKE_BTRFS are the caller's.
said=
status=
rewrite() {
  : > "$work/calls"
  said=$(PATH=$tools:$PATH FAKE_CALLS=$work/calls PI_COMPRESS_LOGS_PRESSURE=$quiet \
    FAKE_ENCODED=${FAKE_ENCODED:-no} FAKE_REOPEN=${FAKE_REOPEN:-no} FAKE_BTRFS=${FAKE_BTRFS:-same} bash -euo pipefail "$script" "$1" 2>&1)
  status=$?
}

# A log written to yesterday and one under 64 KiB are never rewritten.
sessions=$work/young
mkdir -p "$sessions/project"
head -c 100000 /dev/zero > "$sessions/project/young.jsonl"
head -c 1000 /dev/zero > "$sessions/project/small.jsonl"
touch -d '5 days ago' "$sessions/project/small.jsonl"
rewrite "$sessions"
[ "$status" -eq 0 ] || fail "over a young log and a small one the rewrite ended $status, not 0: $said"
[ -s "$work/calls" ] && fail "a young log or a small one reached btrfs: $(cat "$work/calls")"

# A log already compressed enough is left alone.
sessions=$(old_log packed)
FAKE_ENCODED=yes rewrite "$sessions"
[ "$status" -eq 0 ] || fail "over a compressed log the rewrite ended $status, not 0: $said"
[ -s "$work/calls" ] && fail "a log already compressed reached btrfs"

# A log a session reopens after the walk found it is left alone.
sessions=$(old_log resumed)
FAKE_REOPEN=yes rewrite "$sessions"
[ "$status" -eq 0 ] || fail "over a log reopened after the walk the rewrite ended $status, not 0: $said"
[ -s "$work/calls" ] && fail "a log reopened after the walk reached btrfs"

# A failed rewrite ends 1 and says so, the log as it was.
sessions=$(old_log failing)
before=$(cksum < "$sessions/project/failing.jsonl")
FAKE_BTRFS=fail rewrite "$sessions"
[ "$status" -eq 1 ] || fail "a failed rewrite ended $status, not 1: $said"
case $said in
*"couldn't rewrite $sessions/project/failing.jsonl compressed"*) ;;
*) fail "a failed rewrite didn't say it couldn't rewrite the log: $said" ;;
esac
[ "$(cksum < "$sessions/project/failing.jsonl")" = "$before" ] || fail "a failed rewrite changed the log"

# A rewrite that leaves a byte different, length and time kept, ends 1.
sessions=$(old_log flipped)
FAKE_BTRFS=flip rewrite "$sessions"
[ "$status" -eq 1 ] || fail "a rewrite that changed a byte ended $status, not 1: $said"
case $said in
*"flipped.jsonl read "*" before its rewrite and "*" after"*) ;;
*) fail "a rewrite that changed a byte didn't name the log: $said" ;;
esac

# A log a session writes to during its rewrite is passed over.
sessions=$(old_log reopened)
FAKE_BTRFS=append rewrite "$sessions"
[ "$status" -eq 0 ] || fail "a log written to during its rewrite ended the run $status, not 0: $said"
case $said in
*"reopened.jsonl was written to during its rewrite"*"rewrote 0 logs"*) ;;
*) fail "a log written to during its rewrite wasn't passed over: $said" ;;
esac

# A rewrite that keeps the bytes ends 0 and counts the log.
sessions=$(old_log kept)
rewrite "$sessions"
[ "$status" -eq 0 ] || fail "a rewrite that kept the log ended $status, not 0: $said"
case $said in
*"rewrote 1 logs compressed"*) ;;
*) fail "a rewrite that kept the log didn't count it: $said" ;;
esac
grep -q 'kept.jsonl$' "$work/calls" || fail "the log to rewrite never reached btrfs"

if [ "$failures" -gt 0 ]; then
  echo "$failures of the rewrite's rules didn't hold" >&2
  exit 1
fi
echo "the rewrite keeps each of its rules"

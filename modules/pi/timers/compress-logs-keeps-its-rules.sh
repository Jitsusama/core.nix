# shellcheck shell=bash
# Runs the nightly log rewrite's own script, the file it's given, under
# bash with the options writeShellApplication gives it, over a folder of
# logs in a build's sandbox, with stand-ins for what needs a real btrfs or
# a real disk: a stat that says the folder is on btrfs, a filefrag that
# reports each log's extents as compressed or not, a btrfs that rewrites a
# log as each case asks, and a sync, a dd and a sleep that note each call.
# It passes when the rewrite keeps each of its rules: a young log and a
# small one never reach btrfs; one already compressed enough, or reopened
# after the walk found it, is left alone; a failed rewrite ends 1, saying
# so; a rewrite that leaves a byte different ends 1, as does one that
# grows the log keeping its time, or grows it with a byte changed; a log
# written to during its rewrite is passed over, failing nothing; a sync
# that fails ends 1; a rewrite waits while other work waits on the disk;
# and a rewrite that keeps the bytes ends 0, counting it, once the log is
# written out and its pages dropped. The built program can't be run this
# way, since its own tools come first on its PATH.
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
[ "$1" = -v ] || exit 2
[ "${FAKE_REOPEN:-no}" = yes ] && echo '{"type":"message"}' >> "${*: -1}"
flags=last,eof
[ "${FAKE_ENCODED:-no}" = yes ] && flags=last,encoded,eof
echo "Filesystem type is: 9123683e"
echo " ext:     logical_offset:        physical_offset: length:   expected: flags:"
echo "   0:        0..      31:     123..       154:     32:             $flags"
EOF

# Each call is noted, then the log is rewritten as FAKE_BTRFS says:
# failing, flipping a byte with its length and time kept, appending as a
# session would, appending with its time kept, appending over a changed
# byte, or keeping it as it was. Asked anything but a compressed
# defragment, it fails.
echo "#!$BASH" > "$tools/btrfs"
cat >> "$tools/btrfs" << 'EOF'
log=${*: -1}
[ "$*" = "filesystem defragment -czstd $log" ] || exit 2
echo "$log" >> "$FAKE_CALLS"
flip() {
  printf 'X' | dd of="$log" bs=1 seek=0 conv=notrunc status=none
}
cp -p "$log" "$log.was"
case ${FAKE_BTRFS:-same} in
fail) exit 1 ;;
flip) flip && touch -r "$log.was" "$log" ;;
append) echo '{"type":"message"}' >> "$log" ;;
grow-same-time) echo '{"type":"message"}' >> "$log" && touch -r "$log.was" "$log" ;;
flip-append) sleep 1.1 && flip && echo '{"type":"message"}' >> "$log" ;;
*) ;;
esac
rm "$log.was"
EOF

# sync and a dd that drops a log's pages note their call; any other dd
# runs as it is. FAKE_SYNC=fail fails the sync.
echo "#!$BASH" > "$tools/sync"
cat >> "$tools/sync" << EOF
echo "sync \$*" >> "\$FAKE_CALLS"
[ "\${FAKE_SYNC:-ok}" = fail ] && exit 1
exit 0
EOF
echo "#!$BASH" > "$tools/dd"
cat >> "$tools/dd" << EOF
case " \$* " in
*" iflag=nocache "*) echo "dd \$*" >> "\$FAKE_CALLS" ;;
*) exec $(command -v dd) "\$@" ;;
esac
EOF
# A wait for the disk: noted, and the disk then stops being busy, so a
# rewrite that waits on it ends rather than hanging the build.
echo "#!$BASH" > "$tools/sleep"
cat >> "$tools/sleep" << EOF
case "\$1" in
5) echo "slept" >> "\$FAKE_CALLS"
  echo "full avg10=0.00 avg60=0.00 avg300=0.00 total=0" > "\$PI_COMPRESS_LOGS_PRESSURE" ;;
*) exec $(command -v sleep) "\$@" ;;
esac
EOF
chmod +x "$tools/stat" "$tools/filefrag" "$tools/btrfs" "$tools/sync" "$tools/dd" "$tools/sleep"

# Nothing waits on the disk here, but where a case says so.
quiet=$work/pressure

failures=0
fail() {
  echo "$*" >&2
  failures=$((failures + 1))
}

# A fresh sessions folder holding one old log past 64 KiB, named $1.
old_log() {
  local sessions=$work/$1
  mkdir -p "$sessions/project"
  # Bash's own substitution over 100 KB takes minutes, so tr makes it.
  head -c 100000 /dev/zero | tr '\0' a > "$sessions/project/$1.jsonl"
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
  echo "full avg10=${FAKE_PRESSURE:-0.00} avg60=0.00 avg300=0.00 total=0" > "$quiet"
  said=$(PATH=$tools:$PATH FAKE_CALLS=$work/calls PI_COMPRESS_LOGS_PRESSURE=$quiet \
    FAKE_ENCODED=${FAKE_ENCODED:-no} FAKE_REOPEN=${FAKE_REOPEN:-no} \
    FAKE_BTRFS=${FAKE_BTRFS:-same} FAKE_SYNC=${FAKE_SYNC:-ok} \
    bash -euo pipefail "$script" "$1" 2>&1)
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
if [ -s "$work/calls" ]; then
  calls=$(cat "$work/calls")
  fail "a young log or a small one reached btrfs: $calls"
fi

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

# A failed rewrite ends 1 and says so.
sessions=$(old_log failing)
FAKE_BTRFS=fail rewrite "$sessions"
[ "$status" -eq 1 ] || fail "a failed rewrite ended $status, not 1: $said"
case $said in
*"couldn't rewrite $sessions/project/failing.jsonl compressed"*) ;;
*) fail "a failed rewrite didn't say it couldn't rewrite the log: $said" ;;
esac

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

# A rewrite that grows the log but keeps its time isn't a session's
# write, so it ends 1.
sessions=$(old_log stretched)
FAKE_BTRFS=grow-same-time rewrite "$sessions"
[ "$status" -eq 1 ] || fail "a rewrite that grew the log, its time kept, ended $status, not 1: $said"

# Nor is one that grows it over a changed byte, its time moved on.
sessions=$(old_log rewritten)
FAKE_BTRFS=flip-append rewrite "$sessions"
[ "$status" -eq 1 ] || fail "a rewrite that grew the log over a changed byte ended $status, not 1: $said"

# A log that can't be written out after its rewrite ends 1, saying so.
sessions=$(old_log unsynced)
FAKE_SYNC=fail rewrite "$sessions"
[ "$status" -eq 1 ] || fail "a failed sync ended $status, not 1: $said"
case $said in
*"couldn't write $sessions/project/unsynced.jsonl out"*) ;;
*) fail "a failed sync didn't say so: $said" ;;
esac

# While other work waits on the disk, the rewrite says so and waits.
sessions=$(old_log patient)
FAKE_PRESSURE=50.00 rewrite "$sessions"
[ "$status" -eq 0 ] || fail "a rewrite that waited on the disk ended $status, not 0: $said"
case $said in
*"waiting while other work waits on the disk"*"rewrote 1 logs"*) ;;
*) fail "a rewrite didn't say it waited on the disk: $said" ;;
esac
grep -qx slept "$work/calls" || fail "a rewrite didn't wait while the disk was busy"

# A rewrite that keeps the bytes ends 0 and counts the log, once it has
# written the log out and dropped its pages to read it back from disk.
sessions=$(old_log kept)
rewrite "$sessions"
[ "$status" -eq 0 ] || fail "a rewrite that kept the log ended $status, not 0: $said"
case $said in
*"rewrote 1 logs compressed"*) ;;
*) fail "a rewrite that kept the log didn't count it: $said" ;;
esac
log=$sessions/project/kept.jsonl
grep -qx "$log" "$work/calls" || fail "the log to rewrite never reached btrfs"
grep -qx "sync -- $log" "$work/calls" || fail "the rewritten log was never written out"
grep -qx "dd if=$log iflag=nocache count=0 status=none" "$work/calls" \
  || fail "the rewritten log's pages were never dropped, so it was read from memory"

if [ "$failures" -gt 0 ]; then
  echo "$failures of the rewrite's rules didn't hold" >&2
  exit 1
fi
echo "the rewrite keeps each of its rules"

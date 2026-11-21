# shellcheck shell=bash
# What the shell has reported (modules/quickshell/Report.qml), as the desktop
# test's `reports` runs it (../desktop.nix): waits until the shell reports
# something starting with PREFIX after the journal's point SINCE, or since
# the boot when SINCE is empty, then prints every report since that point,
# each as "said", a tab and what was said, and last the point it read to, as
# "cursor" and a tab. SECONDS running out ends it with 3, a status of its own
# that heard.TIMED_OUT names, since a read that fails ends it with the
# failing command's, most often 1. heard-test.py runs it against a stand-in
# journal to hold both.
#
# A report is what follows the line's first "report[", as the nested niri's
# reader takes it (modules/quickshell/tests/launcher_on_niri.py), since what
# it shows may say it again. It reads the journal again every fifth of a
# second rather than following it, since journalctl's follower is not always
# woken for an entry: on a busy machine it missed one in 300 that a fresh
# read found. The journal keeps Quickshell's lines as bytes, since it colours
# their level, so they're read as plain text: JSON would give them as
# numbers.
since=$1 seconds=$2 prefix=$3
# Bash's arithmetic would run what a time held, so any time but a number ends it with 2.
[[ $seconds =~ ^[0-9]+$ ]] || exit 2
journal=(journalctl --user --unit=quickshell --no-pager --output=cat)
journal+=(--lines=all --show-cursor)
if [ -n "$since" ]; then journal+=("--after-cursor=$since"); else journal+=(--boot); fi
deadline=$((SECONDS + seconds))
while :; do
  said=$("${journal[@]}")
  if grep -aqF "report[$prefix" <<<"$said"; then break; fi
  [ "$SECONDS" -lt "$deadline" ] || exit 3
  sleep 0.2
done
sed -n -e 's/^-- cursor: /cursor\t/p' \
  -e '/report\[/{s/report\[/\n/;s/^[^\n]*\n\(.*\)\]$/said\t\1/p}' <<<"$said"

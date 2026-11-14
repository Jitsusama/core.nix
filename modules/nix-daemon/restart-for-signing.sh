#!/usr/bin/env bash
# Restarts a Mac's Nix daemon so it reads /etc/nix/signing.conf, only when
# that line differs from the one it was last restarted for, since a restart
# ends the builds it's running. Run by darwin.nix after /etc is written.
#
# Usage: restart-for-signing.sh LABEL READ
#   LABEL  the daemon's launchd label
#   READ   where the line it last read is kept
set -euo pipefail

label=$1
read=$2

line=$(/usr/bin/shasum -a 256 /etc/nix/signing.conf | /usr/bin/cut -d' ' -f1)
if [ "$line" != "$(/bin/cat "$read" 2>/dev/null)" ] &&
  /bin/launchctl print "system/$label" >/dev/null 2>&1; then
  echo "restarting $label, so it signs what it builds" >&2
  /bin/launchctl kickstart -k "system/$label"
  echo "$line" >"$read"
fi

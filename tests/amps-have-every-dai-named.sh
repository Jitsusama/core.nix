# shellcheck shell=bash
# Checks every cs35l56 DAI the SoundWire machine driver names for an
# amplifier is one the amplifiers' driver registers, since one it doesn't
# leaves no sound card at all.
# usage: amps-have-every-dai-named.sh <machine driver> <amplifiers' driver>
set -euo pipefail

machine=$1
codec=$2
named=$({ grep -o '\.dai_name = "cs35l56-[^"]*"' "$machine" || true; } | cut -d '"' -f 2 | sort -u)
if [ -z "$named" ]; then
  echo "The SoundWire machine driver names no cs35l56 DAI, so there's nothing to hold."
  exit 1
fi
missing=
for dai in $named; do
  grep -qF ".name = \"$dai\"" "$codec" || missing="$missing $dai"
done
if [ -n "$missing" ]; then
  echo "The SoundWire machine driver names DAIs no amplifier registers:$missing"
  exit 1
fi

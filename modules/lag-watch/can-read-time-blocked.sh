# shellcheck shell=bash
# Holds both halves lag-watch's time blocked needs, since without either
# every row reads no time blocked and nothing else fails. Run as
# `can-read-time-blocked.sh ASKED CONFIG`, with ASKED what a machine with a
# screen sets kernel.sched_schedstats to and CONFIG core's kernel's
# configuration. It exits non-zero, saying which half is missing.
set -eu

asked=$1
config=$2

if [ "$asked" != 1 ]; then
  echo "a machine with a screen sets kernel.sched_schedstats to $asked, not 1" >&2
  exit 1
fi
if ! grep -qx 'CONFIG_SCHEDSTATS=y' "$config"; then
  echo "core's kernel is built without CONFIG_SCHEDSTATS, so nothing keeps time blocked" >&2
  exit 1
fi

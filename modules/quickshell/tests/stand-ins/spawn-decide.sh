#!/usr/bin/env bash
# The harness's spawn-decide, for tests/sessions-spawn-decides.qml: each
# session id says how its decision goes, as the real one's outcomes are
# (2QZBX9, harness #1406). Each start is noted in $DECIDED, with the
# folder, the quest and the model it was given, and a terminate signal
# heard is noted there too, with when, in milliseconds since the epoch.
set -euo pipefail
: "${DECIDED:?the check names the file each decision is noted in}"

id="" model=""
while [ $# -gt 0 ]; do
	case "$1" in
	--session-id) id="$2" && shift 2 ;;
	--model) model="$2" && shift 2 ;;
	*) shift ;;
	esac
done

# Heard before it's noted as asked, so a signal that comes once the
# check reads the note always finds the trap.
staying=""
trap 'echo "ended $id at $(date +%s%3N)" >>"$DECIDED"; [ -z "$staying" ] || kill "$staying"
	exit 143' TERM
echo "asked $id in $PWD quest=${QUEST_WORKFLOW_AUTOLOAD_ID:-} model=$model" >>"$DECIDED"

case "$id" in
s-hosted)
	echo "opening harbour-background"
	echo "hosted harbour-background s-hosted"
	;;
s-stayed)
	echo "opening harbour-background"
	echo "the host is full"
	exit 3
	;;
s-slow)
	# Says nothing, as a decider that hangs would, until it's ended. bash
	# runs a trap the moment `wait` is signalled, so nothing it waits on
	# holds the note of its ending back.
	sleep infinity &
	staying=$!
	wait "$staying"
	;;
*) exit 1 ;;
esac

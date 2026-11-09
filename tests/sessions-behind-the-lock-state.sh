#!/usr/bin/env bash
# Lays out what tests/sessions-behind-the-lock.qml needs beside the session
# it hands another beside, which its stand-in for the machine's node tells
# of, under the folder given: the realm the node keeps.
set -euo pipefail

state=$1
mkdir -p "$state/pi/agentic-harness.pi" "$state/mycelium"
echo harbour >"$state/mycelium/realm"

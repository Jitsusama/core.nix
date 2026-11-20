#!/usr/bin/env bash
# Proves the MX Brio's relay can start with the plugins its service is
# given, on either picture the Brio sends: its NV12 at 1080p, passed
# through, and its MJPEG, decoded, as on a USB 2 link. The relay's own
# input runs with a test picture in the Brio's place, since no build
# has a camera, so a missing plugin or caps that can't be met fails here
# rather than leaving Joel, with the Brio hidden, no camera at all.
#
# Usage: the-relay-takes-either-format.sh INPUT
set -euo pipefail

input=$1
brio='v4l2src device=/dev/mx-brio'
if [[ $input != "$brio !"* ]]; then
  echo "the relay doesn't read the Brio first: $input"
  exit 1
fi
rest=${input#"$brio"}
picture='videotestsrc num-buffers=5 ! video/x-raw,width=1920,height=1080,framerate=30/1'

export GST_REGISTRY_1_0=$TMPDIR/registry.bin
# The pipelines are split into gst-launch's words, as the relay's are, with
# none of their characters taken for a file's name.
set -f
for source in "$picture,format=NV12" "$picture ! jpegenc"; do
  # shellcheck disable=SC2086 # gst-launch takes a pipeline word by word.
  if ! gst-launch-1.0 -q $source $rest ! fakesink; then
    echo "the relay can't start on: $source"
    exit 1
  fi
done

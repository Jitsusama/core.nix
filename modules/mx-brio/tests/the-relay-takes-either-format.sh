#!/usr/bin/env bash
# Proves the MX Brio's relay can start with the plugins its service is
# given, on either picture the Brio sends: its NV12 at 1080p, passed
# through, and its MJPEG, decoded, as on a USB 2 link. The relay's own
# input runs with a test picture in the Brio's place, since no build
# has a camera, so a missing plugin or caps that can't be met fails here
# rather than leaving Joel, with the Brio hidden, no camera at all. The
# input is made alone too, as the relay makes it, before linking a sink.
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
# The relay makes its input from the description alone and links its own
# sink after, so the input has to be made with nothing after it, which a
# run ending in a sink can't show: the parser takes bare caps only between
# two elements. Run alone, it stops at its unlinked end, and only one
# that can't be made, in part or at all, is called erroneous or said
# not to be constructed.
# shellcheck disable=SC2086 # gst-launch takes a pipeline word by word.
alone=$(gst-launch-1.0 -q $picture,format=NV12 $rest 2>&1 || true)
if [[ $alone == *"erroneous pipeline"* || $alone == *"could not be constructed"* ]]; then
  echo "the relay can't make its input, ending as it does: $alone"
  exit 1
fi

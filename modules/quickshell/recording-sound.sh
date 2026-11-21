# shellcheck shell=bash
# Runs a recording (Recorder.qml) with the sound it carries: Joel's
# microphone and what the desktop plays, mixed into a source of the
# recording's own, which it names to the recording in RECORDING_SOUND.
# No recording carried sound before (Joel, 2026-11-20: "why does a screen
# recording not record audio?").
#
# The mix is a sink no program plays to, with the default source and the
# default sink's monitor looped into it, so nothing is heard twice and no
# feedback reaches the speakers. PipeWire shares a capture with every
# program that asks, so a call in Meet or Tuple keeps the microphone while
# a recording reads it too, and the loops name themselves recording-sound,
# a program a-call-is-on knows is no call. The mix ranks below every sink,
# so it's never a default and the desktop's sound and a call's never land
# in it, and it goes once the recording ends, however it ends but killed
# outright. Where there's no sound server, as on a
# machine without one, the recording runs without sound rather than not at
# all. Its help takes the shape every command of ours does
# (docs/commands.md), by hand.

help() {
  cat << 'EOF'
recording-sound  run a recording with the microphone and the desktop's sound

  Mixes the default microphone and what the default speakers play into
  one source of the recording's own, names it in RECORDING_SOUND, and
  runs the command given. The mix goes once the command ends. Without
  a sound server the command runs with RECORDING_SOUND unset.

EXAMPLES
  recording-sound sh -c 'wf-recorder --audio="$RECORDING_SOUND" -f a.mp4'

EXIT
  the command's own, or 64 when none was given
EOF
}

case "${1:-}" in
-h | --help)
  help
  exit 0
  ;;
"")
  help >&2
  exit 64
  ;;
*) ;;
esac

mix=recording-$$
loaded=()

# Unloads what was loaded, the loops before the sink they feed.
let_go() {
  local index
  for ((index = ${#loaded[@]} - 1; index >= 0; index--)); do
    pactl unload-module "${loaded[index]}" 2> /dev/null || true
  done
  loaded=()
}
trap let_go EXIT
# Stopped itself, it lets the recording finish its file first, since bash
# runs a trap only once the command it waits on is done, and says it was
# interrupted, as the convention's one code for that.
trap 'exit 130' TERM HUP

# Loads one module, keeping its number so it's let go of.
load() {
  local module
  module=$(pactl load-module "$@" 2> /dev/null) || return
  loaded+=("$module")
}

# The default source and the default sink's monitor, read once: a recording
# keeps the sound it began with. pactl joins its words with spaces, so each
# value is quoted for the module's own reading of them.
loop=("source_output_properties='application.process.binary=recording-sound'"
  "sink_input_properties='application.process.binary=recording-sound'"
  source_dont_move=true sink_dont_move=true latency_msec=30)
if microphone=$(pactl get-default-source 2> /dev/null) &&
  speakers=$(pactl get-default-sink 2> /dev/null) &&
  load module-null-sink sink_name="$mix" \
    "sink_properties='device.description=Recording priority.session=0'" &&
  load module-loopback "source='$microphone'" sink="$mix" "${loop[@]}" &&
  load module-loopback "source='$speakers.monitor'" sink="$mix" "${loop[@]}"; then
  export RECORDING_SOUND=$mix.monitor
else
  let_go
  unset RECORDING_SOUND
fi

"$@"

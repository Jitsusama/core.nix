# shellcheck shell=bash
# Wires the IR sensor's link through the IPU7 to a capture node, sets the
# format at each end, and names the capture node /run/ir-camera/source, as
# camera.nix says why. Its help takes the shape every command of ours does
# (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
ir-camera-pipeline  wire the IR camera to a capture node

  Links the IR sensor through the IPU7 to a capture node,
  sets the format at each end, and names the node
  /run/ir-camera/source.

EXAMPLES
  ir-camera-pipeline

EXIT
  0   wired
  1   the sensor hasn't bound, or no capture node follows
  else media-ctl's own
EOF
  exit 0
  ;;
*) ;;
esac

format=Y10_1X10/648x368

media=
for device in /sys/bus/media/devices/media*; do
  model=$(< "$device/model") || continue
  if [ "$model" = ipu7 ]; then
    media=/dev/${device##*/}
  fi
done
sensor=
for node in /sys/class/video4linux/v4l-subdev*; do
  name=$(cat "$node/name")
  case $name in
    "hm1092 "*) sensor=$name ;;
    *) ;;
  esac
done
if [ -z "$media" ] || [ -z "$sensor" ]; then
  echo "The IR sensor hasn't bound to the IPU7." >&2
  exit 1
fi

topology=$(media-ctl -d "$media" -p)
# The entity a pad of an entity's links out to, its first if several.
target() {
  awk -v entity="$1" -v pad="$2:" '
    /^- entity / { inside = index($0, ": " entity " (") > 0; at = "" }
    inside && /^\tpad[0-9]+:/ { at = $1 }
    inside && at == pad && /^\t\t-> "/ { split($0, part, "\""); print part[2]; exit }
  ' <<< "$topology"
}
port=$(target "$sensor" pad0)
capture=$(target "$port" pad1)
if [ -z "$port" ] || [ -z "$capture" ]; then
  echo "No capture node follows $sensor." >&2
  exit 1
fi

media-ctl -d "$media" -l "\"$port\":1 -> \"$capture\":0 [1]"
media-ctl -d "$media" -V "\"$sensor\":0 [fmt:$format]"
media-ctl -d "$media" -V "\"$port\":0 [fmt:$format]"
node=$(media-ctl -d "$media" -e "$capture")
ln -sfn "$node" /run/ir-camera/source

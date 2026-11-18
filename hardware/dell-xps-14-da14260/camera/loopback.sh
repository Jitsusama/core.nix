# shellcheck shell=bash
# Makes the camera's loopback, or takes the one already there, and writes
# its node to a file for the relay and the holders watch to read.
#
# The relay's stop deletes its loopback, but a delete fails while a program
# still holds it open, as a call or a test left waiting on a camera giving no
# frames does, and each start used to add another. The one left behind
# stayed in every program's list of cameras with no picture, and Chrome
# keeps a camera by its node, so a call could take the dead one. Taking the
# loopback already there keeps its node across restarts. A program still
# holding it sees frames again once the relay writes to it, since the
# module gives a closed writer's side to the next one, at the format the
# reader holds.
#
# Of several of the same name, every one but the lowest-numbered is
# deleted where it can be. When one is held, the lowest is deleted too if
# nothing holds it, and the held one is kept, since it is the one a program
# is waiting on; the others held are tried again at the next start. A
# loopback taken keeps the options it was made with, so a change to add's
# options reaches it once a stop deletes it. LOOPBACK_SYSFS names the folder
# the loopbacks are listed in, for tests; the service never sets it. Its
# help takes the shape every command of ours does (docs/commands.md), by
# hand.

case ${1:-} in
-h | --help)
  cat << 'EOF'
camera-loopback  take the camera's loopback, or make one

EXAMPLES
  camera-loopback 'Built-in Camera' FILE -b 4 -x 1
      take Built-in Camera's loopback, or add one with
      those options; its node goes in FILE

EXIT
  0   taken or added
  1   v4l2loopback-ctl couldn't add one
  64  asked wrongly
EOF
  exit 0
  ;;
*) ;;
esac

set -euo pipefail

if [ $# -lt 2 ]; then
  echo "✕ camera-loopback needs a name and a file" >&2
  echo "  try  camera-loopback --help" >&2
  exit 64
fi
name=$1
file=$2
shift 2
sysfs=${LOOPBACK_SYSFS:-/sys/devices/virtual/video4linux}

# The loopbacks of this name, lowest-numbered first: a sparse array, indexed
# by number, lists its indices in order.
found=()
for card in "$sysfs"/video*/name; do
  [ -e "$card" ] || continue
  read -r card_name < "$card" || true
  if [ "$card_name" = "$name" ]; then
    folder=${card%/name}
    found[${folder##*/video}]=1
  fi
done
nodes=()
for number in "${!found[@]}"; do
  nodes+=("/dev/video$number")
done

# Deletes a loopback, saying so, or says v4l2loopback-ctl's reason when the
# module refuses; deleted says which. It never fails, so set -e still holds
# wherever it's called.
delete() {
  local why
  if why=$(v4l2loopback-ctl delete "$1" 2>&1); then
    echo "Deleted $1, a $name left from before."
    deleted=yes
  else
    echo "Couldn't delete $1, a $name left from before ($why); trying again next start."
    deleted=no
  fi
}

if [ "${#nodes[@]}" -eq 0 ]; then
  kept=$(v4l2loopback-ctl add -n "$name" "$@")
else
  held=()
  for node in "${nodes[@]:1}"; do
    delete "$node"
    if [ "$deleted" = no ]; then held+=("$node"); fi
  done
  kept=${nodes[0]}
  if [ "${#held[@]}" -gt 0 ]; then
    delete "$kept"
    if [ "$deleted" = yes ]; then kept=${held[0]}; fi
  fi
  echo "Taking $name at $kept, already there."
fi

echo "$kept" > "$file"

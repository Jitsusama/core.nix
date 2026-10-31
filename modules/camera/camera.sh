# shellcheck shell=bash disable=SC2154 # The device, look and script come from home.nix's header.
# Shows the camera in a window to take a picture or a video with
# (camera.lua), as home.nix says. The header written above this (./home.nix)
# gives device, the camera's device or the file naming it, look, mpv's
# script options for the theme, and script, camera.lua's path.
#
# A camera plugged in, such as the one on a desk, is taken over the
# machine's own while it's there, as Joel chose: its USB port says it's
# removable, where a built-in camera's says fixed. CAMERA_ROOT puts /dev
# and /sys somewhere else, for the check that tests this (checks.nix).
#
# Usage: camera [picture | video]

mode=${1:-picture}
case $mode in
  picture | video) ;;
  *)
    echo "usage: camera [picture | video]" >&2
    exit 2
    ;;
esac
# A camera made at boot is named by a file.
[ -c "$device" ] || device=$(cat "$device")
root=${CAMERA_ROOT:-}
for node in "$root"/dev/v4l/by-id/usb-*-video-index0; do
  [ -e "$node" ] || continue
  # The node's USB interface, whose parent is the camera's USB device.
  usb=$root/sys/class/video4linux/$(basename "$(readlink "$node")")/device/..
  if [ "$(cat "$usb/removable" 2>/dev/null)" = removable ]; then
    device=$node
    break
  fi
done
# Drawn on the GPU through Wayland, or in shared memory where there's
# no GPU to draw with. It idles between sources, so the camera can pass
# to a recording and back (camera.lua). A picture's frame is kept
# uncompressed, since mpv waits on it and the JPEG made of it is what's
# kept: compressed, a busy machine took seconds over each, and the keys
# waited behind it. The window opens once camera.lua has bound the keys,
# which mpv waits for before it plays, and before the camera wakes: made
# sooner, a key pressed as it appeared on a busy machine met no binding
# and was lost.
exec mpv "av://v4l2:$device" \
  --vo=gpu-next,wlshm --gpu-context=wayland \
  --title=Camera --wayland-app-id=camera --force-window=yes \
  --profile=low-latency --untimed --cache=no --hwdec=no --aid=no \
  --vf=hflip --osc=no --input-default-bindings=no \
  --keep-open=no --idle=yes --save-position-on-quit=no --resume-playback=no \
  --screenshot-png-compression=0 \
  --script="$script" \
  --script-opts="$look",camera-mode="$mode",camera-device="$device"

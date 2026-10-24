# shellcheck shell=bash disable=SC2154 # The device, look and script come from home.nix's header.
# Shows the camera in a window to take a picture or a video with
# (camera.lua), as home.nix says. The header written above this (./home.nix)
# gives device, the camera's device or the file naming it, look, mpv's
# script options for the theme, and script, camera.lua's path.
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
# Drawn on the GPU through Wayland, or in shared memory where there's
# no GPU to draw with. It idles between sources, so the camera can pass
# to a recording and back (camera.lua).
exec mpv "av://v4l2:$device" \
  --vo=gpu-next,wlshm --gpu-context=wayland \
  --title=Camera --wayland-app-id=camera --force-window=immediate \
  --profile=low-latency --untimed --cache=no --hwdec=no --aid=no \
  --vf=hflip --osc=no --input-default-bindings=no \
  --keep-open=no --idle=yes --save-position-on-quit=no --resume-playback=no \
  --script="$script" \
  --script-opts="$look",camera-mode="$mode",camera-device="$device"

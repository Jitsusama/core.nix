# shellcheck shell=bash
# Starts a shell showing a full-screen window (uncached-pictures-let-go.qml,
# as the shell's root) with a red picture read uncached from a file, then
# writes the file over green and moves the picture to a second window. The
# second window shows green only where the first texture let the file's
# pixels go and they were read again; Qt 6.12.0 keeps them beside the
# texture, 20 MB at 2880 by 1800, and shows red there, and with
# QSG_TRANSIENT_IMAGES lets them go for good and shows nothing. It's told by
# what's shown, not by what the process weighs, since on a builder's
# software rasteriser the texture is the process's own memory too and
# QSG_TRANSIENT_IMAGES weighed no lighter there; on a laptop's GPU each let
# go saved the picture's whole 19.8 MB (2026-11-15).
# usage: uncached-pictures-let-go-on-niri.sh <shell folder> <logs>
set -euo pipefail

shell=$1
logs=$2
# Through Qt's GPU path, as the laptop's shell draws, rather than Qt's
# software renderer, whose textures share the file's pixels.
export QT_QUICK_BACKEND=rhi QSG_RHI_BACKEND=opengl

# As large as the laptop's screen, as the wallpaper's picture is.
picture=$PWD/picture.png
after=$PWD/after.png
magick -size 2880x1800 xc:red "$picture"
magick -size 2880x1800 xc:lime "$after"
second=$PWD/second-window.png

said=$logs/shell.log
# It waits 15 s, and Mesa's rasteriser draws slowly on a loaded builder.
if ! PICTURE=$picture AFTER=$after SECOND_WINDOW=$second timeout 120 qs -p "$shell" > "$said" 2>&1; then
  echo "The shell failed, or didn't finish in 120 s:" >&2
  cat "$said" >&2
  exit 1
fi

means=$(magick "$second" -format '%[fx:mean.r] %[fx:mean.g]' info:)
read -r red green <<< "$means"
echo "The second window shows red $red and green $green."
if awk -v red="$red" 'BEGIN { exit !(red > 0.5) }'; then
  echo "The second window shows the picture first read: its file's pixels were kept." >&2
  exit 1
fi
if ! awk -v green="$green" 'BEGIN { exit !(green > 0.5) }'; then
  echo "The second window shows nothing of the picture: it wasn't read again." >&2
  exit 1
fi

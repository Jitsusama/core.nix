# shellcheck shell=bash
# Starts a shell showing a full-screen window (flat-windows-hold-no-depth.qml,
# as the shell's root) drawn flat and then drawn with Qt Quick's depth and
# stencil buffers, and checks the flat one holds neither, and that niri
# shows it drawn in order, its square over its picture. It draws through
# OpenGL on Mesa's software rasteriser, whose buffers are the process's own
# memory: a depth and stencil buffer of 2880 by 1800 pixels is 20,250 KB,
# and a window without them, as QSG_NO_DEPTH_BUFFER and
# QSG_NO_STENCIL_BUFFER make every window, weighed 19,300 to 20,500 KB less
# on a laptop (2026-11-15). On a loaded builder the saving read 9,408 to
# 12,696 KB over six runs, and each side alone moved by 3 MB, so it fails
# at less than a quarter of the buffer, clear of both.
# usage: flat-windows-hold-no-depth-on-niri.sh <shell folder> <logs> <Memory.js>
set -euo pipefail

shell=$1
logs=$2
cp "$3" "$shell/Memory.js"
# Through Qt's GPU path, as the laptop's shell draws, rather than Qt's
# software renderer, which has no depth buffer. Mesa's shader cache off,
# since a first run that fills it weighed 10 MB more than the next.
export QT_QUICK_BACKEND=rhi QSG_RHI_BACKEND=opengl MESA_SHADER_CACHE_DISABLE=true
# With one arena, as the shell's unit holds jemalloc to (home.nix), since
# each thread's arena of a builder's many keeps what it freed apart.
export MALLOC_CONF=narenas:1

# A picture of colours throughout, red at its left, so it's told apart from
# the square set over it.
picture=$PWD/picture.png
magick -size 1800x2880 gradient:red-blue -rotate -90 "$picture"
buffers=$((2880 * 1800 * 4))

# What the shell weighs, in KB, drawn flat where FLAT is 1, with what niri
# shows of it once it's weighed written to the file named, if one is.
weighs() {
  local said=$logs/shell-${2}.log
  local weight=""
  FLAT=$1 PICTURE=$picture QML_XHR_ALLOW_FILE_READ=1 timeout 120 qs -p "$shell" > "$said" 2>&1 &
  local shell_pid=$!
  # Each run waits 10 s before it weighs, and Mesa's rasteriser draws
  # slowly on a loaded builder.
  for _ in $(seq 600); do
    weight=$(sed -n 's/.*flat windows: weighs \([0-9]*\) KB.*/\1/p' "$said")
    [ -n "$weight" ] && break
    kill -0 "$shell_pid" 2> /dev/null || break
    sleep 0.1
  done
  if [ -z "$weight" ]; then
    echo "The shell said nothing of its weight, $2:" >&2
    cat "$said" >&2
    exit 1
  fi
  if [ -n "${3:-}" ]; then
    niri msg action screenshot-screen --show-pointer false --path "$3" >&2
  fi
  if ! wait "$shell_pid"; then
    echo "The shell failed, or didn't finish in 120 s, $2:" >&2
    cat "$said" >&2
    exit 1
  fi
  echo "$weight"
}

flat=$(weighs 1 flat "$PWD/flat.png")
deep=$(weighs 0 deep "$PWD/deep.png")
saved=$((deep - flat))
echo "Drawn flat, the window weighs $saved KB less: $flat KB against $deep."
if ! awk -v saved="$saved" -v buffers="$buffers" 'BEGIN { exit !(saved * 1024 >= buffers / 4) }'; then
  echo "The flat window holds depth and stencil buffers still." >&2
  exit 1
fi

# niri writes its screenshot as the next frame is drawn, so it's waited for.
for _ in $(seq 100); do
  [ -s flat.png ] && [ -s deep.png ] && break
  sleep 0.1
done
# The square is lime, and the picture at the left of it red.
square=$(magick flat.png -format '%[fx:p{w/2,h/2}.g > 0.8 && p{w/2,h/2}.r < 0.2]' info:)
left=$(magick flat.png -format '%[fx:p{w/16,h/2}.r > 0.5 && p{w/16,h/2}.g < 0.2]' info:)
if [ "$square" != 1 ] || [ "$left" != 1 ]; then
  echo "niri doesn't show the flat window's square over its picture (square $square, left $left)." >&2
  exit 1
fi

# The see-through window, its card blue near its foot at the left, is drawn
# at all, so the two can't match by showing neither.
card=$(magick flat.png -format '%[fx:p{w/72,h*0.18}.b > 0.8 && p{w/72,h*0.18}.r < 0.2]' info:)
if [ "$card" != 1 ]; then
  echo "niri doesn't show the see-through window's card drawn flat." >&2
  exit 1
fi
# Drawn flat, both windows show what they show with depth and stencil, to
# within a rounding of colour: a clip or a layer that needed either would
# cut or order something else. The few pixels allowed are a curve's or a
# word's edges rounded apart, far fewer than a word or a corner drawn wrong.
differ=$(magick compare -metric AE -fuzz 2% flat.png deep.png null: 2>&1 || true)
differ=${differ%% *}
if ! awk -v differ="$differ" 'BEGIN { exit !(differ + 0 == differ && differ <= 200) }'; then
  echo "Drawn flat, the windows show $differ pixels other than drawn with depth." >&2
  magick compare -fuzz 2% flat.png deep.png "$logs/differ.png" || true
  exit 1
fi
echo "Drawn flat, the windows show what they show with depth, $differ pixels apart."

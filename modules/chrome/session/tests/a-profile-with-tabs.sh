# shellcheck shell=bash
# Builds a Chrome profile with five tabs open in two windows, for
# commands-agree to set chrome-session's bare run at each width from: one
# in front of each window, one with a long title, and Chrome's own
# settings, which keeps its title alone. It writes the session file as
# Chrome does (src/session.rs reads it, src/written.rs writes it for the
# crate's own tests), since a kept binary file says nothing of what it
# holds.
# usage: a-profile-with-tabs.sh <folder to build it in>
set -euo pipefail

out=$1

# The file as printf escapes, written whole by one printf at the end. Each
# piece adds to it in place rather than through a subshell, so a failing
# piece stops the script rather than leaving a hole in the file.
file='SNSS'
escaped=''

# Each number as the file holds it: little-endian, four bytes or eight.
int() {
  local value=$1
  printf -v escaped '\\x%02x\\x%02x\\x%02x\\x%02x' $((value & 255)) $((value >> 8 & 255)) \
    $((value >> 16 & 255)) $((value >> 24 & 255))
  file+=$escaped
}
long() {
  int $(($1 & 0xffffffff))
  int $(($1 >> 32 & 0xffffffff))
}

# A string's bytes, ASCII only, padded to four as a pickle holds it, or
# the same as UTF-16, two bytes a letter.
bytes() {
  local text=$1 wide=${2:-} at size=${#1}
  for ((at = 0; at < ${#text}; at++)); do
    printf -v escaped '\\x%02x' "'${text:at:1}"
    file+=$escaped
    [[ -z $wide ]] || file+='\x00'
  done
  [[ -z $wide ]] || size=$((size * 2))
  for ((at = size; at % 4 != 0; at++)); do file+='\x00'; done
}

# A command's head: its size counting its id, then its id. What it
# carries, `size` bytes, follows.
command() {
  local id=$1 size=$2
  printf -v escaped '\\x%02x\\x%02x\\x%02x' $(((size + 1) & 255)) $(((size + 1) >> 8)) "$id"
  file+=$escaped
}

# Chrome's clock counts microseconds from 1601.
to_unix=11644473600000000

# A tab of a window, at a place in it, on one page, looked at `looked`
# seconds after 1970.
tab() {
  local window=$1 tab=$2 index=$3 address=$4 title=$5 looked=$6
  command 0 8
  int "$window"
  int "$tab"
  command 2 8
  int "$tab"
  int "$index"
  local pickle=$((12 + (${#address} + 3) / 4 * 4 + 4 + (${#title} * 2 + 3) / 4 * 4 + 8))
  command 6 $((4 + pickle))
  int "$pickle"
  int "$tab"
  int 0
  int ${#address}
  bytes "$address"
  int ${#title}
  bytes "$title" wide
  int 0
  int 0
  command 7 8
  int "$tab"
  int 0
  command 21 16
  int "$tab"
  int 0
  long $((to_unix + looked * 1000000))
}

int 3
tab 1 10 0 https://example.org/guide 'A Guide to Everything' 1700000300
tab 1 11 1 https://news.example.com/ 'The News' 1700000200
tab 1 12 2 chrome://settings/ Settings 1700000100
tab 2 20 0 https://docs.example.net/book/chapter-twelve \
  'Chapter Twelve: Where the Long Title Runs On Past the Width of Any Terminal' 1700000250
tab 2 21 1 https://shop.example.com/cart 'Your Cart' 1700000050
# Which tab is in front of each window, by its place there.
command 8 8
int 1
int 0
command 8 8
int 2
int 1

mkdir -p "$out/Sessions"
printf '%b' "$file" >"$out/Sessions/Session_13300000000000000"

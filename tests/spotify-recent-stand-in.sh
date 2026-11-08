#!/usr/bin/env bash
# Stands in for spotify-recent (modules/spotify/spotify-recent) in
# tests/spotify-program.qml: notes each run in $HOME/asked, a line each,
# the way Spotify stands first, and answers as spotify-recent does for it.
#
#   spotify-recent-stand-in.sh well|signed-out|offline|nowhere [ASKED...]
#
# Well, it says what played lately, a playlist playing and the rest two
# hours ago, the liked songs' count and two playlists, and plays. Signed
# out and offline, it exits as spotify-recent does then, 2 and 4; with
# nowhere to play, it reads as well does but a play exits 3.
set -euo pipefail

stands=$1
shift
printf '%s %s\n' "$stands" "$*" >>"$HOME/asked"

case $stands in
signed-out) exit 2 ;;
offline) exit 4 ;;
esac

ago=$(date -u -d '-2 hours' +%Y-%m-%dT%H:%M:%S.000Z)
case ${1:-} in
'')
  printf 'spotify:playlist:p9\tplaylist\tMorning\tJoel\tplaying\n'
  printf 'spotify:album:a2\talbum\tTwo\tSinger, Guest\t%s\n' "$ago"
  printf 'spotify:collection:tracks\tcollection\tLiked Songs\t\t%s\n' "$ago"
  ;;
liked) printf '1234\n' ;;
playlists)
  printf 'spotify:playlist:p1\tplaylist\tFocus\tJoel\t42\n'
  printf 'spotify:playlist:p3\tplaylist\tRoad trip\tSpotify\t1\n'
  ;;
play) [ "$stands" != nowhere ] || exit 3 ;;
*) exit 64 ;;
esac

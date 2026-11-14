#!/usr/bin/env bash
# Stands in for spotify-recent (modules/spotify/spotify-recent) in
# tests/spotify-program.qml: notes each run in $HOME/asked, a line each,
# the way Spotify stands first, and answers as spotify-recent does for it.
#
#   spotify-recent-stand-in.sh well|signed-out|offline|nowhere [ASKED...]
#
# Well, it says what played lately, a playlist playing, an album two hours
# ago and a show the day before, the liked songs' count, two playlists,
# an album, an artist and no shows, and plays. Signed out and offline, it
# exits as spotify-recent does then, 78 and 69; with nowhere to play, it
# reads as well does but a play exits 66.
set -euo pipefail

stands=$1
shift
printf '%s %s\n' "$stands" "$*" >>"$HOME/asked"

case $stands in
signed-out) exit 78 ;;
offline) exit 69 ;;
esac

ago=$(date -u -d '-2 hours' +%Y-%m-%dT%H:%M:%S.000Z)
yesterday=$(date -u -d '-26 hours' +%Y-%m-%dT%H:%M:%S.000Z)
case ${1:-} in
'')
  printf 'spotify:playlist:p9\tplaylist\tMorning\tJoel\tplaying\t46\n'
  printf 'spotify:album:a2\talbum\tTwo\tSinger, Guest\t%s\t1\n' "$ago"
  printf 'spotify:show:s1\tshow\tThe Show\tA Publisher\t%s\t1212\n' "$yesterday"
  ;;
liked) printf '1234\n' ;;
playlists)
  printf 'spotify:playlist:p1\tplaylist\tFocus\tJoel\t\t42\n'
  printf 'spotify:playlist:p3\tplaylist\tRoad trip\tSpotify\t\t1\n'
  ;;
albums) printf 'spotify:album:b1\talbum\tKid A\tRadiohead\t\t10\n' ;;
artists) printf 'spotify:artist:r1\tartist\tArtist\t\t\t\n' ;;
shows) ;;
play) [ "$stands" != nowhere ] || exit 66 ;;
*) exit 64 ;;
esac

# shellcheck shell=bash
# xdg-open, handing what it's given to gio, which opens each kind of file in
# the app declared for it (nixos.nix says why it replaces the generic one).
# It answers xdg-open's own --help, --manual and --version with its usage.
# usage: xdg-open { file | URL }
usage="usage: xdg-open { file | URL }, opened through gio"
if [ "$#" -eq 0 ]; then
  echo "$usage" >&2
  exit 1
fi
case "$1" in
  --help | --manual | --version)
    echo "$usage"
    exit 0
    ;;
esac
exec gio open "$@"

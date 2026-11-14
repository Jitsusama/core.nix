# shellcheck shell=bash
# xdg-open, handing what it's given to gio, which opens each kind of file in
# the app declared for it (nixos.nix says why it replaces the generic one).
# It answers xdg-open's own --help, --manual and --version with its help,
# in the shape every command of ours takes (docs/commands.md), and with
# nothing to open exits 1, as xdg-utils' own does for every program that
# calls it; gio's 2 for what it can't open is xdg-utils' code for a file
# that isn't there, so it's passed on rather than made one of our table's.
if [ "$#" -eq 0 ]; then
  printf '%s\n' "xdg-open needs a file or an address to open" \
    "  try  xdg-open --help" >&2
  exit 1
fi
case "$1" in
  -h | --help | --manual | --version)
    cat << 'EOF'
xdg-open  open a file or an address in its app

  Each kind opens in the app declared for it, through gio.

EXAMPLES
  xdg-open notes.pdf            the PDF in its reader
  xdg-open https://example.com  the page in the browser

EXIT
  0   opened
  1   nothing was given to open
  2   it couldn't be opened
EOF
    exit 0
    ;;
esac
exec gio open "$@"

# shellcheck shell=bash
# Checks a mycelium: link a page hands the desktop, as the realm's map
# does, reaches the launcher's entry and is run as the entry runs it.
# xdg-open must find the entry by what it says it opens, beside a list of
# defaults a browser wrote, which names none for these links; gio, and so
# the portal, must find it only through niri's own list. The entry's
# program then runs, built to ask a stand-in qs, over links like the map's
# and ones it mustn't pass on, and a second link within a few seconds of
# the first.
# usage: mycelium-links-open-in-the-launcher.sh <desktop entry> <niri's defaults>
#   <mycelium-link asking a stand-in qs>
set -euo pipefail

entry=$1
defaults=$2
asking_stand_in=$3
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"

mkdir -p data/applications config
mkdir -m 700 runtime
cp "$entry" data/applications/mycelium-link.desktop
printf '[Default Applications]\nx-scheme-handler/https=browser.desktop\n' > config/mimeapps.list
cp "$defaults" config/niri-mimeapps.list

# Runs a command as the account's desktop would, with only these files to
# read.
desktop() {
  HOME=$work XDG_DATA_HOME=$work/data XDG_DATA_DIRS=$work/none XDG_CONFIG_HOME=$work/config \
    XDG_CONFIG_DIRS=$work/none XDG_CURRENT_DESKTOP=niri XDG_RUNTIME_DIR=$work/runtime "$@"
}

found=$(desktop xdg-mime query default x-scheme-handler/mycelium)
if [ "$found" != mycelium-link.desktop ]; then
  echo "xdg-open opens mycelium links with '$found'"
  exit 1
fi
said=$(desktop gio mime x-scheme-handler/mycelium 2>&1)
if ! grep -qxF 'Default application for “x-scheme-handler/mycelium”: mycelium-link.desktop' \
  <<< "$said" || grep -q WARNING <<< "$said"; then
  echo "gio opens mycelium links with: $said"
  exit 1
fi
# The entry runs the home's own build, asking the shell's qs, so it's
# checked by name; the stand-in's build below is the same program.
command=$(sed -n 's|^Exec=\(/nix/store/[^ ]*/bin/mycelium-link\) %u$|\1|p' "$entry")
if [ -z "$command" ] || [ ! -x "$command" ]; then
  echo "the entry runs '$(sed -n 's|^Exec=||p' "$entry")', not mycelium-link"
  exit 1
fi

# The stand-in qs writes down what it's asked here.
export ASKED=$work/asked
failed=0
# Opens a link as the entry would, the last one's time forgotten, and checks
# what it asked of qs: nothing, where it's refused.
opening() {
  rm -f asked runtime/mycelium-link.last
  desktop "$asking_stand_in" "$1" 2> /dev/null || true
  if [ "$(cat asked 2> /dev/null)" != "$2" ]; then
    echo "'$1' asked '$(cat asked 2> /dev/null)', not '$2'"
    failed=1
  fi
}
opening 'mycelium:cell/DAHTNJ' 'ipc call launcher talkTo DAHTNJ'
opening 'MYCELIUM://cell/DAHTNJ/talk?say=hi#now' 'ipc call launcher talkTo DAHTNJ'
opening 'mycelium:network/repo.core-nix' 'ipc call launcher network repo.core-nix'
opening 'mycelium:cell/-h' ''
opening 'mycelium:cell/DAH%20TNJ' ''
opening 'mycelium:cell/DAHTNJ;reboot' ''
opening 'mycelium:session/DAHTNJ' ''
opening 'https://example.com/mycelium:cell/DAHTNJ' ''

# A page's second link, right after the first, opens nothing more.
rm -f asked runtime/mycelium-link.last
desktop "$asking_stand_in" 'mycelium:cell/DAHTNJ'
desktop "$asking_stand_in" 'mycelium:network/repo.core-nix' 2> /dev/null
if [ "$(cat asked)" != 'ipc call launcher talkTo DAHTNJ' ]; then
  echo "two links at once asked: $(cat asked)"
  failed=1
fi
[ "$failed" = 0 ]

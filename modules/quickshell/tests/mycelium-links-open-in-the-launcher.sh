# shellcheck shell=bash
# Checks a mycelium: link a page hands the desktop, as the realm's map
# does, reaches the launcher's entry and is run as the entry runs it.
# xdg-open must find the entry by what it says it opens, beside a list of
# defaults a browser wrote, which names none for these links; gio, and so
# the portal, must find it only through niri's own list. The portal opens
# a link straight in its app only when exactly one is registered for the
# scheme, so the entry must be, in the profile's applications, whose
# mimeinfo.cache is written as the home's is. The entry's
# program then runs, built to ask a stand-in qs, over links like the map's
# and ones it mustn't pass on, and a second link within a few seconds of
# the first.
# usage: mycelium-links-open-in-the-launcher.sh <desktop entry> <niri's defaults>
#   <mycelium-link asking a stand-in qs> <Launcher.qml> <mycelium-link's main.rs>
set -euo pipefail

entry=$1
defaults=$2
asking_stand_in=$3
launcher=$4
link_source=$5
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"

mkdir -p data/applications profile/share/applications config
mkdir -m 700 runtime
cp "$entry" profile/share/applications/mycelium-link.desktop
# As Home Manager writes the profile's cache.
update-desktop-database profile/share/applications
printf '[Default Applications]\nx-scheme-handler/https=browser.desktop\n' > config/mimeapps.list
cp "$defaults" config/niri-mimeapps.list

# Runs a command as the account's desktop would, with only these files to
# read.
desktop() {
  HOME=$work XDG_DATA_HOME=$work/data XDG_DATA_DIRS=$work/profile/share \
    XDG_CONFIG_HOME=$work/config \
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
# The portal counts the apps gio recommends; with none, it asks Open With
# rather than opening the link.
recommended=$(sed -n '/^Recommended applications:$/,$p' <<< "$said" | sed -n 's/^\t//p')
if [ "$recommended" != mycelium-link.desktop ]; then
  echo "the portal would offer '$recommended' for mycelium links, not mycelium-link alone: $said"
  exit 1
fi
# The entry runs the home's own build, asking the shell's qs, so it's
# checked by name; the stand-in's build below is the same program.
command=$(sed -n 's|^Exec=\(/nix/store/[^ ]*/bin/mycelium-link\) %u$|\1|p' "$entry")
if [ -z "$command" ] || [ ! -x "$command" ]; then
  runs=$(sed -n 's|^Exec=||p' "$entry")
  echo "the entry runs '$runs', not mycelium-link"
  exit 1
fi

# The stand-in qs writes down what it's asked here, each argument in
# brackets.
export ASKED=$work/asked
failed=0
# Opens a link as the entry would, the last one's time forgotten, and checks
# what it asked of qs: nothing, where it's refused.
opening() {
  rm -f asked runtime/mycelium-link.last
  # A refused link exits as it should, so its exit isn't what's checked.
  # shellcheck disable=SC2310
  desktop "$asking_stand_in" "$1" 2> /dev/null || true
  said=$(cat asked 2> /dev/null) || said=
  if [ "$said" != "$2" ]; then
    echo "'$1' asked '$said', not '$2'"
    failed=1
  fi
}
opening 'mycelium:cell/DAHTNJ' '[ipc][call][launcher][talkTo][DAHTNJ]'
opening 'MYCELIUM://cell/DAHTNJ/talk?say=hi#now' '[ipc][call][launcher][talkTo][DAHTNJ]'
# A notice's whole cell link, naming where its session runs and what in it
# the notice is about, reaches the same session by its address alone.
opening 'mycelium:cell/DAHTNJ@kestrel#notice:12' '[ipc][call][launcher][talkTo][DAHTNJ]'
opening 'mycelium:cell/DAHTNJ@kestrel/go#notice:12' '[ipc][call][launcher][goTo][DAHTNJ]'
opening 'mycelium:network/repo.core-nix' '[ipc][call][launcher][network][repo.core-nix]'
# The system page's Open in the Launcher, on a machine's row.
opening 'mycelium:machine/osprey?from=system' '[ipc][call][launcher][machine][osprey]'
# The map's Start New Work from a cell: the words decoded once, the cell
# kept only as an address, each after a fixed word so neither is a flag.
opening 'mycelium:sprout?words=tidy%20the%20rows%20%26%20more&from=TERN02' \
  '[ipc][call][launcher][newWork][words:tidy the rows & more][from:TERN02]'
opening 'mycelium:sprout?words=-rf+now&from=tern02' \
  '[ipc][call][launcher][newWork][words:-rf now][from:]'
opening 'mycelium:sprout?from=TERN02' ''
opening 'mycelium:sprout?words=tidy&words=reboot' ''
# Its Start New Work Here, on a machine's drawer: the machine after a fixed
# word too, in the launcher's call naming one, and a misshaped one refused.
opening 'mycelium:sprout?words=tidy&on=osprey' \
  '[ipc][call][launcher][newWorkOn][words:tidy][from:][on:osprey]'
opening 'mycelium:sprout?words=tidy&from=TERN02&on=cloud' \
  '[ipc][call][launcher][newWorkOn][words:tidy][from:TERN02][on:cloud]'
opening 'mycelium:sprout?words=tidy&on=-h' ''
# Code's page's Open it in the launcher, on a repository's row.
opening 'mycelium:repository/github:someone/thing' \
  '[ipc][call][launcher][code][github:someone/thing]'
opening 'mycelium:repository/github:someone/thing;reboot' ''
opening 'mycelium:cell/dahtnj' ''
opening 'mycelium:cell/DAHTNJ@kes;trel' ''
opening 'mycelium:cell/-h' ''
opening 'mycelium:cell/DAH%20TNJ' ''
opening 'mycelium:cell/DAHTNJ;reboot' ''
opening 'mycelium:session/DAHTNJ' ''
opening 'https://example.com/mycelium:cell/DAHTNJ' ''

# A page's second link, right after the first, opens nothing more.
rm -f asked runtime/mycelium-link.last
desktop "$asking_stand_in" 'mycelium:cell/DAHTNJ'
desktop "$asking_stand_in" 'mycelium:network/repo.core-nix' 2> /dev/null
said=$(cat asked)
if [ "$said" != '[ipc][call][launcher][talkTo][DAHTNJ]' ]; then
  echo "two links at once asked: $said"
  failed=1
fi

# The stand-in answers any call, so each function the program calls must
# be one the launcher's IpcHandler has; qs refuses any other, and the link
# opens nothing.
handles=$(sed -n '/^    IpcHandler {$/,/^    }$/p' "$launcher" |
  sed -n 's/^        function \([A-Za-z]*\)(.*/\1/p')
calls=$(sed -n '/^    fn call(&self)/,/^    }$/p' "$link_source" | grep -oE '"[A-Za-z]+"' |
  tr -d '"' | sort -u)
if [ "$(wc -l <<< "$calls")" -lt 7 ]; then
  echo "found too few of mycelium-link's calls to judge: $calls"
  failed=1
fi
for function in $calls; do
  if ! grep -qxF "$function" <<< "$handles"; then
    echo "mycelium-link calls $function, which the launcher's IpcHandler hasn't"
    failed=1
  fi
done
[ "$failed" = 0 ]

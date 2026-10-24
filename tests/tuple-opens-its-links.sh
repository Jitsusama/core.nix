# shellcheck shell=bash
# Checks a call link opened in a browser, which comes back as tuple://join,
# reaches Tuple's desktop entry and is run as the entry runs it. xdg-open
# must find the entry by what it says it opens, beside a list of defaults a
# browser wrote, which names none for Tuple's links; gio, and so the portal,
# must find it only through niri's own list. The entry's command then runs
# with stand-ins for tuple and notify-send, over links like the page's and
# ones it mustn't run.
# usage: tuple-opens-its-links.sh <desktop entry> <niri's defaults> <tuple>
set -euo pipefail

entry=$1
defaults=$2
tuple=$3
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"

mkdir -p data/applications config
cp "$entry" data/applications/tuple.desktop
printf '[Default Applications]\nx-scheme-handler/https=browser.desktop\n' > config/mimeapps.list
cp "$defaults" config/niri-mimeapps.list

# Runs a command as the account's desktop would, with only these files to
# read.
desktop() {
  HOME=$work XDG_DATA_HOME=$work/data XDG_DATA_DIRS=$work/none XDG_CONFIG_HOME=$work/config \
    XDG_CONFIG_DIRS=$work/none XDG_CURRENT_DESKTOP=niri "$@"
}

found=$(desktop xdg-mime query default x-scheme-handler/tuple)
if [ "$found" != tuple.desktop ]; then
  echo "xdg-open opens Tuple's links with '$found'"
  exit 1
fi
said=$(desktop gio mime x-scheme-handler/tuple 2>&1)
if ! grep -qxF 'Default application for “x-scheme-handler/tuple”: tuple.desktop' <<< "$said" \
  || grep -q WARNING <<< "$said"; then
  echo "gio opens Tuple's links with: $said"
  exit 1
fi

# The entry's command, with the stand-ins in tuple's and notify-send's
# places.
command=$(sed -n 's|^Exec=\(.*\) %u$|\1|p' "$entry")
sed -e "s|$tuple|$work/tuple|g" -e "s|/nix/store/[^ ]*/bin/notify-send|$work/notify-send|g" \
  "$command" > opens
for stand_in in tuple notify-send; do
  printf '#!%s\necho "%s $*" > "%s/asked"\n' "$BASH" "$stand_in" "$work" > "$stand_in"
done
chmod +x opens tuple notify-send

failed=0
# Opens a link as the entry would, and checks what it asked of the stand-ins.
opening() {
  rm -f asked
  ./opens "$1"
  if [ "$(cat asked)" != "$2" ]; then
    echo "'$1' asked '$(cat asked)', not '$2'"
    failed=1
  fi
}
refused="notify-send --app-name=Tuple Tuple can't open that link"
opening 'tuple://join?call_url_slug=U7BVv2' 'tuple call join https://tuple.app/c/U7BVv2'
opening 'tuple://join?from=web&call_url_slug=a_b-9' 'tuple call join https://tuple.app/c/a_b-9'
opening 'tuple://join?call_url_slug=a;b' "$refused tuple://join?call_url_slug=a;b"
opening 'tuple://join?call_url_slug=*' "$refused tuple://join?call_url_slug=*"
opening 'tuple://auth?code=abc' "$refused tuple://auth?code=abc"
opening "" 'tuple start'
[ "$failed" = 0 ]

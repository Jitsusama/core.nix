# shellcheck shell=bash
# Says whether something asks the screen to stay on and unlocked: Keep Awake
# (../quickshell/home.nix) or call-transcripts while a call is on, each
# holding logind's lock against going idle under its own name. It exits 0,
# saying which, when one does, and 1 when nothing does, so
# `screen-kept || lock-screen` locks unless one does. Any other hold of that
# lock, such as mycelium's while it holds work, only keeps the machine from
# sleeping, and keeps no lock off. A logind that can't be asked keeps
# nothing off either, so the screen still locks. Its help takes the shape
# every command of ours does (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
screen-kept  say whether the screen is asked to stay on

  Keep Awake or a call's transcripts keep it on and
  unlocked, each holding logind's idle lock by its own
  name; it says which on stderr. Nothing else keeps it,
  and a logind that can't be asked keeps nothing.

EXAMPLES
  screen-kept || lock-screen

EXIT
  0   something keeps the screen on
  1   nothing does
EOF
  exit 0
  ;;
*) ;;
esac

inhibitors=$(busctl --json=short call org.freedesktop.login1 /org/freedesktop/login1 \
  org.freedesktop.login1.Manager ListInhibitors) || exit 1
holders=$(jq -r '.data[0][] | select(.[3] == "block" and (.[0] | split(":") | index("idle"))) | .[1]' \
  <<< "$inhibitors") || exit 1

while IFS= read -r who; do
  case $who in
    "Keep Awake" | call-transcripts)
      echo "the screen stays: $who holds it" >&2
      exit 0
      ;;
    *) ;;
  esac
done <<< "$holders"
exit 1

# shellcheck shell=bash disable=SC2154 # $setup comes from nixos.nix's header.
# Sets the consoles' font and keymap as systemd-vconsole-setup does, for the
# run udev starts each time a graphics driver takes the console over. That
# run picks a console that looks free and asks it for its font, and a
# compositor can take the console for graphics in between, as the
# greeter's niri does once the splash lets go: the kernel then refuses the
# question, and the setup failed the boot's unit for a console that wants
# no font. A failure on a console now drawing graphics is that, said and
# done with; each login prompt sets its own console's font anyway
# (nixos.nix). Any other failure is the setup's.
# The header written above this (./nixos.nix) names the setup by path. Its
# help takes the shape every command of ours does (docs/commands.md), by
# hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
console-setup  set a console's font and keymap

  As systemd-vconsole-setup does, for the run udev starts
  as a graphics driver takes the console over. A console
  taken for graphics meanwhile wants no font, so that
  isn't a failure.

EXAMPLES
  console-setup /dev/tty1

EXIT
  0   set, or the console was taken for graphics
  else the setup's own
EOF
  exit 0
  ;;
*) ;;
esac

"$setup" "$@" && exit 0
status=$?
console=${1:-/dev/tty1}
if [[ $(kbdinfo -C "$console" getmode 2>/dev/null) == graphics ]]; then
  echo "$console was taken for graphics while being set up, so it wants no font" >&2
  exit 0
fi
exit "$status"

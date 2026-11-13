# shellcheck shell=bash
# niri-deadline on|off: whether niri renders each frame as late as it can
# and still make the next vblank (nixos.nix's renders-by-the-deadline.patch),
# which Joel tries by turns on his own screen. It asks the running niri over
# IPC (render-deadline-action.patch), so neither way needs a restart, and it
# lasts until niri stops; niri starts with it off. Its help and failures
# take the shape every command of ours does (docs/commands.md), by hand.

case "${1:-}" in
on) want=true ;;
off) want=false ;;
-h | --help)
  cat << 'EOF'
niri-deadline  whether niri renders each frame as late as it can

EXAMPLES
  niri-deadline on           render by the deadline, until niri stops
  niri-deadline off          render as soon as something changes

EXIT
  0   done
  1   the niri running is older than the one installed
  64  asked wrongly
EOF
  exit 0
  ;;
*)
  printf '%s\n' "niri-deadline needs on or off" "  try  niri-deadline --help" >&2
  exit 64
  ;;
esac

# shellcheck disable=SC2154 # NIRI comes from nixos.nix's runtimeEnv.
if ! "$NIRI" msg action set-render-deadline "$want"; then
  printf '%s\n' "the niri running is older than the one installed" \
    "  it takes this only once niri restarts" >&2
  exit 1
fi

if [ "$want" = true ]; then
  echo "niri renders by its deadline now; niri-deadline off puts it back"
else
  echo "niri renders as soon as something changes, as it always has"
fi

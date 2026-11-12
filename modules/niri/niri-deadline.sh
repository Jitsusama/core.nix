# shellcheck shell=bash
# niri-deadline on|off: whether niri renders each frame as late as it can
# and still make the next vblank (nixos.nix's renders-by-the-deadline.patch),
# which Joel tries by turns on his own screen. It asks the running niri over
# IPC (render-deadline-action.patch), so neither way needs a restart, and it
# lasts until niri stops; niri starts with it off.

case "${1:-}" in
on) want=true ;;
off) want=false ;;
*)
  echo "niri-deadline: say on or off" >&2
  exit 2
  ;;
esac

# shellcheck disable=SC2154 # NIRI comes from nixos.nix's runtimeEnv.
if ! "$NIRI" msg action set-render-deadline "$want"; then
  echo "niri-deadline: the niri running predates the one installed;" \
    "it takes this only once niri restarts" >&2
  exit 1
fi

if [ "$want" = true ]; then
  echo "niri renders by its deadline now; niri-deadline off puts it back"
else
  echo "niri renders as soon as something changes, as it always has"
fi

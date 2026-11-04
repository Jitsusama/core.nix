# shellcheck shell=bash disable=SC2154 # The programs and their arguments come from nixos.nix's header.
# Runs the greeter where everything it writes goes in the login's runtime
# folder, since the greeter's account has no home. When it stops without
# saying it signed someone in, tuigreet asks instead. greetd gives a greeter
# the console to write on, which tuigreet needs, so niri and the greeter
# write to the journal instead, as greeter.
#
# It doesn't stop at the first failure: a compositor that falls over is
# exactly when tuigreet has to ask.
# The header written above this (./nixos.nix) names busctl, kbdinfo,
# mktemp, rm, sleep and systemd-cat by path, the compositor's command and
# its config, and tuigreet's command.

if [ -z "${XDG_RUNTIME_DIR:-}" ] || [ ! -w "$XDG_RUNTIME_DIR" ]; then
  XDG_RUNTIME_DIR=$("$mktemp" -d /tmp/greeter.XXXXXXXX)
  export XDG_RUNTIME_DIR
fi
export XDG_CACHE_HOME=$XDG_RUNTIME_DIR/cache XDG_STATE_HOME=$XDG_RUNTIME_DIR/state
export XDG_DATA_HOME=$XDG_RUNTIME_DIR/data
"$rm" -f "$XDG_RUNTIME_DIR/greeter-launching"
"$systemd_cat" --identifier=greeter "${compositor[@]}" -c "$niri_config"
if [ -e "$XDG_RUNTIME_DIR/greeter-launching" ]; then
  exit 0
fi
echo "greeter: it stopped without signing anyone in, so tuigreet asks" >&2
# greetd leaves the console in graphics for the compositor, and logind
# sets it back to text only once a compositor that took the screen lets go
# of it, so one that fell over before taking it would leave tuigreet asking
# where nothing shows. The greeter takes the screen itself, which fails
# while a compositor still holds it, and lets go as busctl ends; logind then
# sets text, here or for the compositor, on its own time, which the greeter
# waits on for up to five seconds.
"$busctl" call org.freedesktop.login1 /org/freedesktop/login1/session/auto \
  org.freedesktop.login1.Session TakeControl b false 2>/dev/null
for _ in {1..50}; do
  [ "$("$kbdinfo" getmode)" = text ] && break
  "$sleep" 0.1
done
exec "${tuigreet[@]}"

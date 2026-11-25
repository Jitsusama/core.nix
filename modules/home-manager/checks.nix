# The checks of home-manager, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  ...
}:
let
  home = account graphical;
  timed = lib.filter (lib.hasSuffix ".service") (
    lib.mapAttrsToList (name: timer: timer.Timer.Unit or "${name}.service") home.systemd.user.timers
  );
  services = map (lib.removeSuffix ".service") timed;
  service = name: home.systemd.user.services.${name} or { };
  restarted = lib.filter (name: ((service name).Unit.X-SwitchMethod or null) != "keep-old") services;
  # A timer whose service the account doesn't write would get one of only
  # the switch's setting, which would hide the unit it means to run.
  bare = lib.filter (name: !((service name).Service ? ExecStart)) services;
in
onLinux {
  # A switch leaves each run a timer started be, so none stops part way or
  # holds the switch while a new one runs.
  a-switch-leaves-timed-runs-be =
    holds "a-switch-leaves-timed-runs-be"
      "A switch restarts what these timers run, and waits on each run: ${lib.concatStringsSep ", " restarted}"
      (services != [ ] && restarted == [ ]);

  # Every timer runs a service the account writes in full, so the switch's
  # setting is never a unit of its own.
  every-timer-runs-a-written-service =
    holds "every-timer-runs-a-written-service"
      "These timers run a service the account doesn't write: ${lib.concatStringsSep ", " bare}"
      (bare == [ ]);
}

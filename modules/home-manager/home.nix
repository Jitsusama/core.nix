# What every account's home-manager does, whatever the machine.
{ config, lib, ... }:
let
  # The service each timer runs: the one its Timer section names, or the
  # one of its own name, as systemd has it.
  timed = map (lib.removeSuffix ".service") (
    lib.filter (lib.hasSuffix ".service") (
      lib.mapAttrsToList (name: timer: timer.Timer.Unit or "${name}.service") config.systemd.user.timers
    )
  );
  # A timed service the account doesn't write would be written here with
  # only the switch's setting, hiding the unit the timer means to run.
  # home-manager gives every service an empty ExecStart, so an empty one is
  # one the account doesn't write.
  bare = lib.filter (
    name:
    lib.elem (config.systemd.user.services.${name}.Service.ExecStart or [ ]) [
      [ ]
      ""
      null
    ]
  ) timed;
in
{
  programs.home-manager.enable = true;
  # Tools that install themselves for one user put their commands here.
  home.sessionPath = [ "$HOME/.local/bin" ];

  # A switch leaves a run a timer started be, and the timer's next run takes
  # the new unit. Restarting it instead stops a run part way and makes the
  # switch wait for a whole new one, and a run can take minutes on a busy
  # machine: a switch that restarted the delivery cycle's five-minute run,
  # under load, outlasted the five minutes the account's setup is given and
  # went back to the system before it (2026-11-25). A service that must
  # restart at a switch says so with its own X-SwitchMethod. A run that
  # hangs is no longer ended by a switch, so a timed service bounds its own
  # run with TimeoutStartSec where one can hang.
  systemd.user.services = lib.genAttrs timed (_: {
    Unit.X-SwitchMethod = lib.mkDefault "keep-old";
  });

  assertions = [
    {
      assertion = bare == [ ];
      message = "These timers run a service the account doesn't write, which the switch's setting would hide: ${lib.concatStringsSep ", " bare}";
    }
  ];
}

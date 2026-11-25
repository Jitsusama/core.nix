# What every account's home-manager does, whatever the machine.
{ config, lib, ... }:
let
  cfg = config.jitsusama.timedRuns;
  # The unit file a service name is written as: a template's instance,
  # such as backup@home, is written as its template, backup@.
  written =
    name:
    let
      parts = lib.splitString "@" name;
    in
    if lib.length parts == 2 then "${lib.head parts}@" else name;
  # The service each timer runs: the one its Timer section names, or the
  # one of its own name, as systemd has it, less those a machine says the
  # account doesn't write.
  timed = lib.subtractLists cfg.unwritten (
    lib.unique (
      map (name: written (lib.removeSuffix ".service" name)) (
        lib.filter (lib.hasSuffix ".service") (
          lib.mapAttrsToList (name: timer: timer.Timer.Unit or "${name}.service") config.systemd.user.timers
        )
      )
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
  options.jitsusama.timedRuns.unwritten = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    description = ''
      Services a timer runs that the account doesn't write, such as a
      package's own unit, by name without .service. The switch's setting
      and the hour-long bound on a run leave them out, so neither hides the unit the
      timer runs.
    '';
  };

  config = {
    programs.home-manager.enable = true;
    # Tools that install themselves for one user put their commands here.
    home.sessionPath = [ "$HOME/.local/bin" ];

    # A switch leaves a run a timer started be, and the timer's next run
    # takes the new unit. Restarting it instead stops a run part way and
    # makes the switch wait for a whole new one, and a run can take minutes
    # on a busy machine: a switch that restarted the delivery cycle's
    # five-minute run, under load, outlasted the five minutes the account's
    # setup is given and went back to the system before it (2026-11-25). A
    # service that must restart at a switch says so with its own
    # X-SwitchMethod.
    #
    # A switch no longer ends a run that hangs, and a timer starts no run
    # while its last is still going, so each timed run is given an hour to
    # end unless its service says its own TimeoutStartSec. The longest timed
    # run in a day of a loaded machine's journal took 28 minutes, a
    # ten-minute timer's, so an hour ends only a run that has stopped.
    systemd.user.services = lib.genAttrs timed (_: {
      Unit.X-SwitchMethod = lib.mkDefault "keep-old";
      Service.TimeoutStartSec = lib.mkDefault "1h";
    });

    assertions = [
      {
        assertion = bare == [ ];
        message = "A timer runs each of these services, which the account doesn't write, so the switch's setting would hide them: ${lib.concatStringsSep ", " bare}. Write each, or name it in jitsusama.timedRuns.unwritten.";
      }
    ];
  };
}

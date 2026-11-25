# What every account's home-manager does, whatever the machine.
{ config, lib, ... }:
let
  cfg = config.jitsusama.timedRuns;
  # The unit file a service name is written as: a template's instance,
  # such as backup@home, is written as its template, backup@. An instance
  # the account writes as a unit of its own is still read as its template:
  # reading the account's services to tell would make their names depend
  # on themselves, which Nix can't evaluate. No timer in core runs one.
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
  services = config.systemd.user.services;
  # A timed service saying something the defaults below would quietly
  # overrule: a legacy restart or reload at a switch under the default
  # keep-old, or a TimeoutSec whose start half the default hour replaces.
  overruled = lib.filter (
    name:
    let
      unit = services.${name}.Unit or { };
      service = services.${name}.Service or { };
    in
    (
      (unit.X-SwitchMethod or null) == "keep-old"
      && ((unit.X-RestartIfChanged or null) == true || (unit.X-ReloadIfChanged or null) == true)
    )
    || (service ? TimeoutSec && (service.TimeoutStartSec or null) == "1h" && service.TimeoutSec != "1h")
  ) (lib.subtractLists bare timed);
  # A name the machine says the account doesn't write that it does write.
  written' = lib.filter (
    name:
    !(lib.elem (services.${name}.Service.ExecStart or [ ]) [
      [ ]
      ""
      null
    ])
  ) cfg.unwritten;
in
{
  options.jitsusama.timedRuns.unwritten = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    description = ''
      Services a timer runs that the account doesn't write, such as a
      package's own unit, by name without .service, and a template's
      instance by its template, such as backup@. The switch's setting and
      the bounds on a run leave them out, so neither hides the unit the
      timer runs. Naming one the account writes is refused.
    '';
  };

  config = {
    programs.home-manager.enable = true;
    # Tools that install themselves for one user put their commands here.
    home.sessionPath = [ "$HOME/.local/bin" ];

    # A switch leaves a run a timer started be, and the timer's next run
    # takes the new unit. Restarting it instead stops a run part way and
    # makes the switch wait for a whole new one, and a run can take minutes
    # on a busy machine: on 2026-11-25 a switch waiting on a restarted
    # timed run outlasted the five minutes the account's setup is given and
    # went back to the system before it. The delivery cycle's services now
    # say keep-old themselves; this gives every other timed service the
    # same. A service that must restart at a switch says so with its own
    # X-SwitchMethod, and one saying X-RestartIfChanged = false already
    # means the same as keep-old.
    #
    # A switch no longer ends a run that hangs, and a timer starts no run
    # while its last is still going, so each timed run has a bound unless
    # its service says its own. A oneshot's run is its start, so
    # TimeoutStartSec ends it after an hour: the longest timed run in a day
    # of a loaded machine's journal took 28 minutes, a ten-minute timer's,
    # so an hour ends only a run that has stopped. Any other type has
    # started once its program runs, so RuntimeMaxSec ends its run, after
    # three hours, since session-backup's says a run can take most of one;
    # a oneshot ignores it. Neither is read for its type, since a service's
    # names can't depend on its own settings.
    systemd.user.services = lib.genAttrs timed (_: {
      Unit.X-SwitchMethod = lib.mkDefault "keep-old";
      Service = {
        TimeoutStartSec = lib.mkDefault "1h";
        RuntimeMaxSec = lib.mkDefault "3h";
      };
    });

    assertions = [
      {
        assertion = bare == [ ];
        message = "A timer runs each of these services, which the account doesn't write, so the switch's setting would hide them: ${lib.concatStringsSep ", " bare}. Write each (an instance as its template, such as backup@), or name it in jitsusama.timedRuns.unwritten if a package writes it.";
      }
      {
        assertion = overruled == [ ];
        message = "These timed services say something the timed-run defaults would overrule: ${lib.concatStringsSep ", " overruled}. One restarting or reloading at a switch says X-SwitchMethod as well; one with a TimeoutSec says TimeoutStartSec as well.";
      }
      {
        assertion = written' == [ ];
        message = "jitsusama.timedRuns.unwritten names services the account writes, which would lose the switch's setting and their bound: ${lib.concatStringsSep ", " written'}. Take them out of it.";
      }
    ];
  };
}

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
  # A legacy key as sd-switch reads it (its ini.rs to_bool): written as
  # home-manager writes the value, then lowered, so a Nix true, an int 1
  # and "Yes" all read as yes. An absent key reads as neither.
  spelled =
    value: lib.toLower (if lib.isBool value then lib.boolToString value else toString value);
  yes =
    value:
    lib.elem (spelled value) [
      "1"
      "yes"
      "y"
      "true"
      "t"
      "on"
    ];
  no =
    value:
    lib.elem (spelled value) [
      "0"
      "no"
      "n"
      "false"
      "f"
      "off"
    ];
  # The switch method a timed oneshot takes unless it says its own: what
  # its legacy keys say, in sd-switch's order (reload, then a restart
  # refused, then a stop refused), else keep-old. A legacy key that says
  # only what sd-switch does anyway, X-RestartIfChanged = true, still reads
  # as restart, since core's default would otherwise overrule it.
  method =
    unit:
    if yes (unit.X-ReloadIfChanged or null) then
      "reload"
    else if no (unit.X-RestartIfChanged or null) then
      "keep-old"
    else if no (unit.X-StopIfChanged or null) then
      "restart"
    else if yes (unit.X-RestartIfChanged or null) then
      "restart"
    else
      "keep-old";
  oneshot = name: (services.${name}.Service.Type or "simple") == "oneshot";
  # A timed oneshot saying no bound of its own. systemd reads TimeoutSec
  # as the start's bound too.
  unbounded = lib.filter (
    name:
    oneshot name
    && !(services.${name}.Service ? TimeoutStartSec)
    && !(services.${name}.Service ? TimeoutSec)
  ) timed;
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
      the bound on a run leave them out, so neither hides the unit the
      timer runs. Naming one the account writes is refused.
    '';
  };

  config = {
    programs.home-manager.enable = true;
    # Tools that install themselves for one user put their commands here.
    home.sessionPath = [ "$HOME/.local/bin" ];

    # A switch leaves a timed oneshot's run be, and the timer's next run
    # takes the new unit. A oneshot is starting for the whole of its run, so
    # restarting it stops a run part way and makes the switch wait for a
    # whole new one, which takes minutes on a busy machine: on 2026-11-25 a
    # switch waiting on a restarted timed run outlasted the five minutes the
    # account's setup is given and went back to the system before it. A
    # service of any other type has started once its program runs, so the
    # switch never waits on it, and it restarts onto the new unit as before.
    # A oneshot's own X-SwitchMethod wins, and its own legacy key is read as
    # that method rather than overruled.
    #
    systemd.user.services = lib.genAttrs timed (name: {
      Unit.X-SwitchMethod = lib.mkIf (oneshot name) (lib.mkDefault (method services.${name}.Unit));
    });

    # A switch no longer ends a oneshot's run that hangs, and a timer starts
    # no run while its last is still going, so a timed oneshot saying no
    # TimeoutStartSec or TimeoutSec of its own ends its run after three
    # hours. The longest timed run in a day of a loaded machine's journal
    # took 28 minutes, and pi-keys, whose sample runs past an hour, says
    # 2h30min itself, so three hours ends only a run that has stopped. A
    # machine's oneshot running longer says its own, "infinity" for none.
    #
    # The bound is a drop-in beside the unit rather than a key in its
    # [Service], since a key there can't read the section it's in: the
    # section's keys are made from its values. systemd reads it as part of
    # the unit, and a template's drop-in reaches each instance.
    xdg.configFile = lib.listToAttrs (
      map (name: {
        name = "systemd/user/${name}.service.d/timed-run.conf";
        value.text = ''
          # core.nix's bound on a timed run: home-manager/home.nix says why.
          [Service]
          TimeoutStartSec=3h
        '';
      }) unbounded
    );

    assertions = [
      {
        assertion = bare == [ ];
        message = "A timer runs each of these services, which the account doesn't write, so the switch's setting would hide them: ${lib.concatStringsSep ", " bare}. Write each (an instance as its template, such as backup@), or name it in jitsusama.timedRuns.unwritten if a package writes it.";
      }
      {
        assertion = written' == [ ];
        message = "jitsusama.timedRuns.unwritten names services the account writes, which would lose the switch's setting and their bound: ${lib.concatStringsSep ", " written'}. Take them out of it.";
      }
    ];
  };
}

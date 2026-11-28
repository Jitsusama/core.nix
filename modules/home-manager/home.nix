# What every account's home-manager does, whatever the machine.
{ config, lib, ... }:
let
  cfg = config.jitsusama.timedRuns;
  runs = import ./timed.nix { inherit lib; };
  services = config.systemd.user.services;
  # The service each timer runs, less those a machine says the account
  # doesn't write, as timed.nix reads them.
  timed = runs.timed {
    timers = config.systemd.user.timers;
    inherit (cfg) unwritten;
  };
  # A timed service the account doesn't write would be written here with
  # only the switch's setting, hiding the unit the timer means to run.
  bare = lib.filter (runs.bare services) timed;
  # A legacy key as sd-switch 0.6.4 reads it (its ini.rs to_bool), the
  # version nixpkgs ships, which checks.nix holds this to: written as
  # home-manager writes the value, then trimmed and lowered, so a Nix
  # true, an int 1 and " Yes" all read as yes. An absent key, or one
  # in no spelling sd-switch knows, reads as neither, as there.
  spelled =
    value:
    let
      said = if lib.isBool value then lib.boolToString value else toString value;
    in
    lib.toLower (lib.trim said);
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
  # its legacy keys say, in sd-switch's order (its unit_file.rs: reload,
  # then a restart refused, then a stop refused), else
  # keep-old. A unit saying only X-RestartIfChanged = true or
  # X-StopIfChanged = true gets no method from sd-switch's legacy
  # reading, so sd-switch stops and starts it; it reads as stop-start
  # here, since core's keep-old would otherwise overrule a unit that
  # asked for its run to be stopped.
  method =
    unit:
    if yes (unit.X-ReloadIfChanged or null) then
      "reload"
    else if no (unit.X-RestartIfChanged or null) then
      "keep-old"
    else if no (unit.X-StopIfChanged or null) then
      "restart"
    else if yes (unit.X-RestartIfChanged or null) || yes (unit.X-StopIfChanged or null) then
      "stop-start"
    else
      "keep-old";
  oneshot = name: (services.${name}.Service.Type or "simple") == "oneshot";
  # A timed oneshot saying no bound of its own that systemd reads, as
  # timed.nix reads it: systemd reads TimeoutSec as the start's bound too,
  # and drops a key it can't read, so a run saying only such a key is
  # bounded here, the drop-in read after the unit.
  unbounded = lib.filter (
    name: oneshot name && (runs.timeout (services.${name}.Service or { })).read == [ ]
  ) timed;
  # Each timed run's timeout key systemd can't read, said where the
  # account is built, so a bound dropped for its spelling is never news
  # first heard from a run killed at three hours.
  unread = lib.concatMap (
    name:
    let
      service = services.${name}.Service or { };
      inherit (runs.timeout service) given read;
    in
    map (
      key:
      "${name}'s ${key} ${builtins.toJSON service.${key}} is one systemd can't read, "
      + "so it is dropped"
      + lib.optionalString (lib.elem name unbounded) " and the run ends after three hours"
    ) (lib.subtractLists read given)
  ) timed;
  # A name the machine says the account doesn't write that it does write.
  written' = lib.filter (name: !runs.bare services name) cfg.unwritten;
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
    # TimeoutStartSec or TimeoutSec of its own that systemd reads ends its
    # run after three hours: a key misspelt is dropped by systemd, so it
    # bounds nothing, and the build warns of it. The longest timed run in a
    # day of a loaded machine's journal took 28 minutes, and pi-keys, whose
    # sample runs past an hour, says 2h30min itself, so three hours ends
    # only a run that has stopped. A machine's oneshot running longer says
    # its own, "infinity" for none.
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

    warnings = unread;

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

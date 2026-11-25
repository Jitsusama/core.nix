# The checks of home-manager, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  exampleLaptop,
  holds,
  lib,
  onLinux,
  ...
}:
let
  # The example laptop's account with `extra` added to it. The laptop
  # imports every module Linux has, so its account has every timer core
  # writes, the delivery cycle's among them.
  account =
    extra:
    (exampleLaptop.extendModules { modules = [ { home-manager.users.joel = extra; } ]; })
    .config.home-manager.users.joel;
  home = account { };
  services = lib.unique (
    map
      (
        name:
        let
          parts = lib.splitString "@" (lib.removeSuffix ".service" name);
        in
        if lib.length parts == 2 then "${lib.head parts}@" else lib.head parts
      )
      (
        lib.filter (lib.hasSuffix ".service") (
          lib.mapAttrsToList (name: timer: timer.Timer.Unit or "${name}.service") home.systemd.user.timers
        )
      )
  );
  method = home: name: (home.systemd.user.services.${name} or { }).Unit.X-SwitchMethod or null;
  # A service may say its own X-SwitchMethod; one that says none is
  # restarted by a switch, which waits on its run. home-manager gives
  # every service the key, null until something sets it.
  unsaid = lib.filter (name: method home name == null) services;
  cadenceKeeps = method home "cadence" == "keep-old";

  # home.nix's assertion as an account with `extra` says it, or null.
  refusal =
    extra:
    lib.findFirst (
      assertion: !assertion.assertion && lib.hasInfix "the switch's setting would hide" assertion.message
    ) null (account extra).assertions;
  # A timer whose service nothing writes.
  stray = {
    systemd.user.timers.stray-probe.Timer.OnCalendar = "daily";
  };
  strayRefused = refusal stray;
  # The same timer, its service named as one the account doesn't write.
  strayExcused = account (stray // { jitsusama.timedRuns.unwritten = [ "stray-probe" ]; });
  # A timer running a template's instance, the template written.
  instance = account {
    systemd.user.timers."probe@one".Timer.OnCalendar = "daily";
    systemd.user.services."probe@".Service.ExecStart = "/run/current-system/sw/bin/true";
  };
  holdsFor = {
    "a timer whose service nothing writes is refused, by name" =
      strayRefused != null && lib.hasInfix "stray-probe" strayRefused.message;
    "a service named in jitsusama.timedRuns.unwritten is left out" =
      refusal (stray // { jitsusama.timedRuns.unwritten = [ "stray-probe" ]; }) == null
      && !(strayExcused.systemd.user.services ? stray-probe);
    "a template's instance keeps its run on the template" =
      method instance "probe@" == "keep-old" && !(instance.systemd.user.services ? "probe@one");
  };
  broken = lib.attrNames (lib.filterAttrs (_: held: !held) holdsFor);
in
onLinux {
  # Every timed service has a switch method, and the delivery cycle's,
  # whose restart held a switch past its time, keeps its run.
  a-switch-leaves-timed-runs-be =
    holds "a-switch-leaves-timed-runs-be"
      "The delivery cycle's service keeps its run: ${lib.boolToString cadenceKeeps}. These timed services have no switch method, so a switch restarts them and waits on each run: ${
        if unsaid == [ ] then "none" else lib.concatStringsSep ", " unsaid
      }"
      (cadenceKeeps && unsaid == [ ]);

  # home.nix's assertion refuses a timer whose service the account doesn't
  # write, lets a machine name such a service, and finds a template's
  # instance on its template.
  every-timer-runs-a-written-service =
    holds "every-timer-runs-a-written-service"
      "home.nix's timers don't hold: ${lib.concatStringsSep "; " broken}"
      (broken == [ ]);
}

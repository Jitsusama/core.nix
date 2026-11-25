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
  # The example laptop imports every module Linux has, so its account has
  # every timer core writes, the delivery cycle's among them.
  home = exampleLaptop.config.home-manager.users.joel;
  services = map (lib.removeSuffix ".service") (
    lib.filter (lib.hasSuffix ".service") (
      lib.mapAttrsToList (name: timer: timer.Timer.Unit or "${name}.service") home.systemd.user.timers
    )
  );
  service = name: home.systemd.user.services.${name} or { };
  # A service may say its own X-SwitchMethod; one that says none is
  # restarted by a switch, which waits on its run. home-manager gives
  # every service the key, null until something sets it.
  unsaid = lib.filter (name: ((service name).Unit.X-SwitchMethod or null) == null) services;
  kept = lib.filter (name: ((service name).Unit.X-SwitchMethod or null) == "keep-old") services;
  # Only home.nix's own assertion, so another module's can't fail this.
  failed = lib.filter (
    assertion: lib.hasPrefix "These timers run" assertion.message && !assertion.assertion
  ) home.assertions;
in
onLinux {
  # Every timed service says how a switch treats it, and the delivery
  # cycle's, whose restart held a switch past its time, keeps its run.
  a-switch-leaves-timed-runs-be =
    holds "a-switch-leaves-timed-runs-be"
      "These timed services say nothing of a switch, so one restarts them and waits on each run, or the delivery cycle's doesn't keep its run: ${lib.concatStringsSep ", " unsaid}"
      (lib.elem "cadence" kept && unsaid == [ ]);

  # home.nix's assertion that every timer runs a service the account writes
  # holds on the laptop that has every timer.
  every-timer-runs-a-written-service =
    holds "every-timer-runs-a-written-service"
      "The example laptop's account fails home.nix's assertion: ${
        lib.concatMapStringsSep "; " (assertion: assertion.message) failed
      }"
      (failed == [ ]);
}

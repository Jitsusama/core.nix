# The checks of pi-packages, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  holds,
  lib,
  macWorkstation,
  onLinux,
  passesClippy,
  pkgs,
  workstation,
  ...
}:
{
  # Each of pi's git packages is updated at a switch, before pi's settings
  # name its link, and in the background every fifteen minutes: by a user
  # timer in background.slice on Linux, which a switch doesn't restart, and
  # by a launchd agent on a Mac.
  pi-packages-swap-in-the-background =
    let
      home = account workstation;
      mac = account macWorkstation;
      service = home.systemd.user.services.pi-packages;
      timer = home.systemd.user.timers.pi-packages.Timer;
      settings = home.home.activation.piTakesCoresSettings;
    in
    holds "pi-packages-swap-in-the-background"
      "pi's packages aren't updated before pi's settings are written, or not in the background on Linux and on a Mac."
      (
        service.Service.Slice == "background.slice"
        && service.Unit.X-RestartIfChanged == false
        && timer.OnUnitActiveSec == "15min"
        && home.home.activation ? piPackagesInstall
        && lib.elem "piPackagesInstall" settings.after
        && mac.launchd.agents.pi-packages.config.StartInterval == 900
      );
}
// onLinux {
  # clippy finds nothing in pi-packages, its tests included, which run as it
  # builds: a live process keeps its release, files unchanged, through an
  # update, and a build that fails or leaves a native module unbuilt is never
  # swapped in.
  pi-packages-passes-clippy = passesClippy "pi-packages-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

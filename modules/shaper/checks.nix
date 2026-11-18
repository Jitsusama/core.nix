# The checks of shaper, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
let
  services = (bareNixos [ self.nixosModules.shaper ]).config.systemd.services;
in
# What follows needs Linux to build or to run, so a Mac leaves it out.
onLinux {
  # The shaper's tests run as it builds: each set, route and queue is read
  # from what the kernel prints, every link to one router counts on its
  # line, a recorded week replays through a controller and is scored, and
  # the one-way controller sets each direction from its own delay.
  shaper-finds-the-rate = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in the shaper, its tests included.
  shaper-passes-clippy = passesClippy "shaper-passes-clippy" (pkgs.callPackage ./package.nix { });

  # A switch restarts the shaper and its recorder in place, in its start
  # phase, rather than stopping them before it activates. The shaper's
  # restart keeps its queues (src/restart.rs); the recorder's keeps the
  # record on the shaper's restart, which on 16 November it was blind to
  # from 20:18:46 to 20:18:53 and from 21:51:52 to 21:52:02.
  shaper-stays-through-a-switch =
    holds "shaper-stays-through-a-switch"
      "A switch would stop the shaper or its recorder first, so its queues or its record go."
      (
        services.shaper.restartIfChanged
        && !services.shaper.stopIfChanged
        && services.shaper-record.restartIfChanged
        && !services.shaper-record.stopIfChanged
      );
}

# The checks of the net's account services, which checks.nix gathers with
# every other folder's.
{
  account,
  bareNixos,
  holds,
  lib,
  self,
  ...
}:
{
  # A switch leaves the net and main's keeper be, and each ends between
  # its rounds or keeps once the build it follows is another. The keeper
  # follows a link of its own, so it has to run the very build linked
  # there: another would end at every start, and a path the link isn't at
  # can't be read, so it would never end and keep running old code.
  merge-net-outlives-a-switch =
    let
      home = account (bareNixos [
        { home-manager.sharedModules = [ self.homeModules.merge-net ]; }
      ]);
      services = home.systemd.user.services;
      keeper = services.changed-checks-keeps-main;
      linked = "${home.xdg.stateFile."changed-checks/installed".source}/bin/changed-checks";
      followed = "${home.xdg.stateHome}/changed-checks/installed/bin/changed-checks";
      started = lib.concatStringsSep " " (lib.toList keeper.Service.ExecStart);
    in
    holds "merge-net-outlives-a-switch"
      "A switch restarts the net or main's keeper, or the keeper follows a build it doesn't run."
      (
        services.merge-net.Unit.X-SwitchMethod == "keep-old"
        && keeper.Unit.X-SwitchMethod == "keep-old"
        && started == "${linked} keep-main --follow ${followed}"
      );
}

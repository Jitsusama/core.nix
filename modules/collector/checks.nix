# The checks of the collector, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  holds,
  onLinux,
  passesClippy,
  pkgs,
  workstation,
  ...
}:
{
  # The collector only keeps what it's sent, so it yields to whatever the
  # account runs, and comes back when it fails.
  collector-stays-in-the-background =
    let
      service = (account workstation).systemd.user.services.collector.Service;
    in
    holds "collector-stays-in-the-background"
      "The collector runs outside background.slice, isn't restarted when it fails, or has no runtime folder for its socket."
      (
        service.Slice == "background.slice"
        && service.Restart == "on-failure"
        && service.RuntimeDirectory == "collector"
      );
}
// onLinux {
  # clippy finds nothing in the collector, its tests included.
  collector-passes-clippy = passesClippy "collector-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

# eero reads the home network's eero from eero's cloud, which answers the
# same at home and away, since the eero itself has no local API
# (src/main.rs says how). Joel signs in once with `eero sign-in`, typing the
# code eero sends him, and the session is sealed for his account on this
# machine. Until then the watch doesn't start.
#
# The watch keeps $XDG_RUNTIME_DIR/eero/status.json, each network's line,
# its last speed test, "Optimize for conferencing and gaming" and how many
# devices are on it, every five minutes, for the shell to read as it reads
# the shaper's. It's background work, a few small requests over the line,
# so it runs in background.slice, behind everything Joel uses.
{ lib, pkgs, ... }:
let
  eero = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ eero ];

  systemd.user.services.eero-watch = {
    Unit = {
      Description = "Keeps the eero's status for the shell";
      # %S is the account's state folder, where `eero sign-in` seals the
      # session.
      ConditionPathExists = "%S/eero/session.cred";
      After = [ "network-online.target" ];
    };
    Service = {
      Slice = "background.slice";
      ExecStart = "${lib.getExe eero} watch";
      # systemd-creds opens the sealed session.
      Environment = [ "PATH=${lib.makeBinPath [ pkgs.systemd ]}" ];
      Restart = "on-failure";
      RestartSec = 60;
    };
    Install.WantedBy = [ "default.target" ];
  };
}

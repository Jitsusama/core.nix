# What clears each of Joel's to-dos by itself: the to-do daemon, a program
# of core's own (src/main.rs), runs each to-do's check and hears its
# signals, and writes what it found to ~/.local/state/mission/verdicts.json,
# which the harness settles each to-do by and the mycelium node serves to
# For You. Every session that needs him writes its to-dos beside it, in
# mission/waiting, through mesh_wait.
#
# A check runs as the account does, on its own PATH, so a to-do's check
# finds what a session's would. A switch's core is read from
# update-to-main's follower's journal, and whether it's past a commit from
# a bare clone of core of the daemon's own, fetched only while a to-do
# waits on a commit it lacks.
#
# It reads Linux's inotify, /proc and sysfs, so it runs only there.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  mission = "${config.xdg.stateHome}/mission";
in
lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
  home.packages = [ package ];

  systemd.user.services.to-do = {
    Unit.Description = "Runs each to-do's check and hears its signals";
    Service = {
      ExecStart = lib.escapeShellArgs [
        (lib.getExe package)
        "${mission}/waiting"
        "${mission}/verdicts.json"
        "${mission}/signins"
        "${config.xdg.stateHome}/to-do/core.git"
      ];
      # The account's own programs for its checks, then git for core's
      # history, journalctl for the follower and timeout for each check.
      Environment = [
        "PATH=${config.home.profileDirectory}/bin:/run/current-system/sw/bin:${
          lib.makeBinPath [
            pkgs.coreutils
            pkgs.git
            pkgs.systemd
          ]
        }"
      ];
      Restart = "always";
      RestartSec = 5;
      Slice = "background.slice";
    };
    Install.WantedBy = [ "default.target" ];
  };
}

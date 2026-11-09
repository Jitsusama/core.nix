# What clears each of Joel's to-dos by itself: the to-do daemon, a program
# of core's own (src/main.rs), runs each to-do's check and hears its
# signals, and writes what it found to ~/.local/state/mission/verdicts.json,
# which the harness settles each to-do by and the mycelium node serves to
# For You. Every session that needs him writes its to-dos beside it, in
# mission/waiting, through mesh_wait.
#
# A check runs with the user manager's environment as it is when the
# check runs, read through systemctl, the one every session's shell
# starts from. The daemon starts before the desktop imports the
# session's PATH (~/.local/bin among it), so a to-do's check finds quick,
# ~/.local/bin and the wrappers as a session's would only by reading it
# afresh; the daemon's own git, journalctl and systemctl are named
# outright instead. A check runs only for a session the harness says is
# outside every sandbox, since here it runs as Joel. Each is stopped, with
# all it started, after 90 seconds, and nothing it started outlives it. A switch's core is read from
# update-to-main's follower's journal, and whether it's past a commit
# from a bare clone of core of the daemon's own, fetched only while a
# to-do waits on a commit it lacks. A switch restarts it, so it runs the
# daemon it was switched to; what it found done stays done.
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
    Unit = {
      Description = "Runs each to-do's check and hears its signals";
      X-SwitchMethod = "restart";
    };
    Service = {
      ExecStart = lib.escapeShellArgs [
        (lib.getExe package)
        "${mission}/waiting"
        "${mission}/verdicts.json"
        "${mission}/signins"
        "${config.xdg.stateHome}/to-do/core.git"
        "--git"
        (lib.getExe pkgs.git)
        "--journalctl"
        "${pkgs.systemd}/bin/journalctl"
        "--systemctl"
        "${pkgs.systemd}/bin/systemctl"
      ];
      Restart = "always";
      RestartSec = 5;
      Slice = "background.slice";
    };
    Install.WantedBy = [ "default.target" ];
  };
}

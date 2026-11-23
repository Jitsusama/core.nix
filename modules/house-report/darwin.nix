# Runs house-report on a Mac that shares a line with a shaped machine, as
# a launchd daemon (system.nix says what it's told). macOS lets a daemon
# reach the local network unasked, but blocks an account's own agent there
# until someone allows it, and asks no one for a program Nix built, so an
# agent would send nothing and say nothing of why. It runs as nobody, since
# it needs only a counter any account may read and a socket to send from.
{ config, lib, ... }:
let
  log = "/var/log/house-report.log";
in
{
  imports = [ ./system.nix ];

  # Made before launchd starts the daemon, and nobody's, so the daemon can
  # write its log whether launchd opens it before or after it takes on
  # nobody's account.
  system.activationScripts.preActivation.text = lib.mkAfter ''
    touch ${log}
    chown nobody:nobody ${log}
    chmod 0644 ${log}
  '';

  launchd.daemons.house-report.serviceConfig = {
    UserName = "nobody";
    GroupName = "nobody";
    # At boot the program, a link into the store, isn't there until the
    # store's volume is mounted, so sh waits for the store first, as
    # nix-darwin's own daemons do.
    ProgramArguments = [
      "/bin/sh"
      "-c"
      "/bin/wait4path /nix/store && exec ${lib.escapeShellArgs config.jitsusama.house-report.arguments}"
    ];
    RunAtLoad = true;
    # launchd waits 10 s between starts should it end.
    KeepAlive = true;
    # Not Background: macOS would mark its socket's traffic background
    # too, and a report held behind the upload it reports on comes late.
    ProcessType = "Standard";
    # Only troubles and their ends, each once, so it grows a line a change.
    StandardErrorPath = log;
  };
}

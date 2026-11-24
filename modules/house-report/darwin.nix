# Runs house-report on a Mac that shares a line with a shaped machine, as
# a launchd daemon (system.nix says what it's told). macOS lets a daemon
# reach the local network unasked, but blocks an account's own agent there
# until someone allows it, and asks no one for a program Nix built, so an
# agent would send nothing and say nothing of why. It runs as nobody, since
# it needs only a counter any account may read and a socket to send from.
{ config, lib, ... }:
let
  log = "/var/log/house-report.log";
  program = lib.escapeShellArgs config.jitsusama.house-report.arguments;
  inherit (config.jitsusama.house-report) keyFile;
in
{
  imports = [ ./system.nix ];

  # Made before launchd starts the daemon, and nobody's, so the daemon can
  # write its log whether launchd opens it before or after it takes on
  # nobody's account.
  system.activationScripts.preActivation.text = lib.mkAfter (
    ''
      touch ${log}
      chown nobody:nobody ${log}
      chmod 0644 ${log}
    ''
    # The daemon reads its key as nobody, so a key in place as the machine
    # switches is made nobody's alone; until one is there, it reports
    # nothing and says so. A key that is a link, by name or a second name
    # for another file, is left as it is, said, so a key file made one to
    # another of root's files never hands that one over; and so is one in
    # a folder not root's alone, where another account could swap it for
    # one between the test and the change. Its folder is read through a
    # link, as a Mac's /etc is one. The Mac's own tools are named, since
    # GNU's stat, which a PATH may name first, reads -f otherwise.
    + lib.optionalString (keyFile != null) ''
      houseReportKey=${lib.escapeShellArg keyFile}
      houseReportFolder=$(/usr/bin/dirname "$houseReportKey")
      houseReportWhy=
      if [ -L "$houseReportKey" ]; then
        houseReportWhy="is a link"
      elif [ -f "$houseReportKey" ] \
        && [ "$(/usr/bin/stat -f %l "$houseReportKey")" != 1 ]; then
        houseReportWhy="has another name, a link to another file"
      elif [ -f "$houseReportKey" ] \
        && [ "$(/usr/bin/stat -L -f %u "$houseReportFolder")" != 0 ]; then
        houseReportWhy="is in a folder that isn't root's"
      elif [ -f "$houseReportKey" ] \
        && [ -n "$(/usr/bin/find -H "$houseReportFolder" -maxdepth 0 -perm +022)" ]; then
        houseReportWhy="is in a folder another account may write"
      elif [ -f "$houseReportKey" ]; then
        /usr/sbin/chown -h nobody:nobody "$houseReportKey"
        /bin/chmod -h 0400 "$houseReportKey"
      fi
      if [ -n "$houseReportWhy" ]; then
        echo "house-report: $houseReportKey $houseReportWhy, so it isn't made nobody's" >&2
      fi
    ''
  );

  launchd.daemons.house-report.serviceConfig = {
    UserName = "nobody";
    GroupName = "nobody";
    # At boot the program, a link into the store, isn't there until the
    # store's volume is mounted, so sh waits for the store first, as
    # nix-darwin's own daemons do.
    ProgramArguments = [
      "/bin/sh"
      "-c"
      "/bin/wait4path /nix/store && exec ${program}"
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

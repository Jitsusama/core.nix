# Keeps a Mac's local name, the one other machines on its network find it by
# as name.local, at the name its configuration declares. macOS renames
# itself name-2, then name-3, when it thinks another machine answers to its
# name, and it can think so from hearing its own answers come back over a
# virtual machine's bridge. Every machine that finds it as name.local then
# loses it until someone sets the name back by hand. nix-darwin sets the
# name only when the system switches, so a job here sets it back whenever
# macOS writes its network settings, and once a minute in case a change
# lands elsewhere, but at most once in five minutes, so a conflict that
# lasts can't have it and macOS trading the name (keep.sh says why). A Mac
# that declares no local name is left as it is.
#
# Nothing here advertises a service to make name.local answer, so it
# answers from other machines only while something else does. macOS puts
# its address records on every interface, loopback aliases too, only while
# some service names the host as its target. A job here did so from 23
# November: on a Mac with about 370 loopback aliases, each became a record
# its name service probed, and when its cable came out, the service spun a
# core on their conflicts. Any service advertised for the host, Remote
# Login's included, brings the records back.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  name = config.networking.localHostName;
in
{
  config.launchd.daemons.local-name = lib.mkIf (name != null) {
    serviceConfig = {
      ProgramArguments = [
        (lib.getExe pkgs.bash)
        "${./keep.sh}"
        name
        "/var/db/local-name.last"
      ];
      # scutil, date and what bash needs, from macOS itself, where only root
      # can write, since the job runs as root.
      EnvironmentVariables.PATH = "/usr/sbin:/usr/bin:/bin";
      WatchPaths = [ "/Library/Preferences/SystemConfiguration/preferences.plist" ];
      StartInterval = 60;
      RunAtLoad = true;
      # Each rename it undoes, with when, and each name it couldn't read or
      # set, so a Mac that keeps renaming itself is seen: each at most once
      # in five minutes, so at most twenty-four lines an hour.
      StandardErrorPath = "/var/log/local-name.log";
    };
  };
}

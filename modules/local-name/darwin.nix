# Keeps a Mac's local name, the one other machines on its network find it by
# as name.local, at the name its configuration declares. macOS renames
# itself name-2, then name-3, when it thinks another machine answers to its
# name, and it can think so from hearing its own answers come back over a
# virtual machine's bridge. Every machine that finds it as name.local then
# loses it until someone sets the name back by hand. nix-darwin sets the
# name only when the system switches, so a job here sets it back whenever
# macOS writes its network settings, and once a minute in case a change
# lands elsewhere, but at most once in five minutes, so a conflict that
# lasts can't have it and macOS trading the name (keep.sh says why).
#
# macOS answers to name.local only while some service it advertises points
# at the machine, and drops the name everywhere once none does. Remote
# Login's ssh is often the only one, and once mDNSResponder had restarted
# launchd was seen not advertising it at all, so a Mac can go unfound with
# every setting right. A second job advertises the machine itself, as a
# workstation on port 9, the way Avahi announces a Linux machine: it claims
# no service the Mac may not run, and it can't clash with Remote Login's.
# dns-sd ends when mDNSResponder goes away, so launchd starts it again and
# the name comes back with the daemon. A Mac that declares no local name is
# left as it is.
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

  config.launchd.daemons.local-name-answers = lib.mkIf (name != null) {
    serviceConfig = {
      ProgramArguments = [
        "/usr/bin/dns-sd"
        "-R"
        name
        "_workstation._tcp"
        "local"
        "9"
      ];
      # Advertising needs no privilege, so it runs as nobody. That leaves it no
      # log of its own, since nobody can't write in /var/log; launchd's own log
      # says each time it ends and starts again.
      UserName = "nobody";
      RunAtLoad = true;
      KeepAlive = true;
    };
  };
}

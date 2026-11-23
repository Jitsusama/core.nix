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
# workstation on port 9, the type and port Avahi announces a Linux machine
# with: it claims no service the Mac may not run, and it can't clash with
# Remote Login's. So the Mac answers to its name on every network it joins,
# as one with Remote Login advertised already does. dns-sd ends when
# mDNSResponder goes away, so launchd starts it again and the name comes
# back with the daemon. A Mac that declares no local name is left as it is.
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
      RunAtLoad = true;
      KeepAlive = true;
      # What dns-sd says is the only word on whether the advertisement took:
      # registered, renamed or refused, and why it ended. A few lines each
      # time it starts, which is at a switch, a boot or mDNSResponder's
      # restart. It runs as root, as the job above does, so launchd can open
      # this log in /var/log and nothing can refuse it the registration.
      StandardOutPath = "/var/log/local-name-answers.log";
      StandardErrorPath = "/var/log/local-name-answers.log";
    };
  };
}

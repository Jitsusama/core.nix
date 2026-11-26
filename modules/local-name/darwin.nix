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
# A second job makes name.local answer the other machines on its network,
# which it does only while some service names the Mac itself as its host.
# macOS then puts its address records on every interface, loopback aliases
# too. A job here did that from 23 November: on a Mac with about 370
# loopback aliases, each became a record its name service probed, and when
# its cable came out, the service spun a core on their conflicts. So the
# job registers one address record of its own instead, naming a host of
# its own, on the one link the network reaches the Mac by (answer.sh says
# how). That host is spelled as the Mac's own name, so whether macOS's name
# service keeps the record apart from its own, rather than bringing them
# back or renaming the Mac, is its to decide, and not yet seen on a Mac.
# A Mac where it doesn't would show it in the logs: keep.sh saying the
# local name was name-2, or dns-sd's own line saying the name is in use.
# Any service advertised for the host, Remote Login's included, would
# bring the records back.
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
      # Reading its link every fifteen seconds, so a Mac that moves to
      # another network answers there within that, and launchd's ten
      # seconds between starts.
      ProgramArguments = [
        (lib.getExe pkgs.bash)
        "${./answer.sh}"
        name
        "15"
      ];
      # scutil, dns-sd, date and sleep, from macOS itself, where only root
      # can write, since the job runs as root.
      EnvironmentVariables.PATH = "/usr/sbin:/usr/bin:/bin";
      # Started again whenever it ends, which it does to answer on a link
      # or address that changed, or when its answer ended.
      KeepAlive = true;
      RunAtLoad = true;
      # Each address it offers, each move and each answer that ended, with
      # when.
      StandardErrorPath = "/var/log/local-name-answers.log";
    };
  };

  # A few lines each time the answer starts is the usual, but an answer
  # that keeps ending, as one refused for a name another machine holds
  # would, runs only until its next read, fifteen seconds, which is past
  # launchd's ten-second wait between starts, so launchd starts it again at
  # once: a few lines every fifteen seconds.
  # So macOS's newsyslog moves each log aside past 512 KB, keeping three
  # before it. It sends no signal, since neither job takes one: launchd
  # opens a job's log anew each time it starts it, which for keep.sh is
  # every minute. answer.sh runs as long as its link holds, so what it
  # writes once newsyslog has moved its log aside lands in the old one,
  # until launchd starts it again.
  config.environment.etc."newsyslog.d/local-name.conf" = lib.mkIf (name != null) {
    text = ''
      # logfilename                    mode count size(KB) when flags
      /var/log/local-name.log          644  3     512      *    N
      /var/log/local-name-answers.log  644  3     512      *    N
    '';
  };
}

# Keeps a Mac's local name, the one other machines on its network find it by
# as name.local, at the name its configuration declares. macOS renames
# itself name-2, then name-3, when it thinks another machine answers to its
# name, and it can think so from hearing its own answers come back over a
# virtual machine's bridge. Every machine that finds it as name.local then
# loses it until someone sets the name back by hand. nix-darwin sets the
# name only when the system switches, so a job here sets it back whenever
# macOS writes its network settings, and once a minute in case a change
# lands elsewhere. A Mac that declares no local name is left as it is.
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
      ];
      # scutil, date and what bash needs, from macOS itself.
      EnvironmentVariables.PATH = "/usr/sbin:/usr/bin:/bin";
      WatchPaths = [ "/Library/Preferences/SystemConfiguration/preferences.plist" ];
      StartInterval = 60;
      RunAtLoad = true;
      # Setting the name writes the same settings it watches; this keeps
      # that from starting it again at once.
      ThrottleInterval = 10;
      # Each rename it undoes, with when, so a Mac that keeps renaming
      # itself is seen.
      StandardErrorPath = "/var/log/local-name.log";
    };
  };
}

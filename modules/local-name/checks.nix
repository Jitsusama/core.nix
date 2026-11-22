# The checks of a Mac's local name, which checks.nix gathers with every other
# folder's; each says what it proves.
{
  bareDarwin,
  holds,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  macNamed =
    settings:
    (bareDarwin [
      self.darwinModules.local-name
      settings
    ]).config.launchd.daemons;
  # A local name of its own, apart from the host name nix-darwin would
  # derive it from, so a job reading the host name instead is caught.
  named = macNamed {
    networking.hostName = "a-mac";
    networking.localHostName = "its-own-name";
  };
  job = named.local-name.serviceConfig or { };
in
{
  # A Mac with a local name runs, as root, keep.sh with that name and its
  # stamp, whenever macOS writes its network settings and once a minute; a
  # Mac with a host name but no local name runs nothing.
  local-name-is-kept-by-a-job =
    holds "local-name-is-kept-by-a-job"
      "A Mac's local name isn't kept by a root job running keep.sh with its local name, watching its network settings, or a Mac with no local name runs one."
      (
        lib.drop 1 (job.ProgramArguments or [ ]) == [
          "${./keep.sh}"
          "its-own-name"
          "/var/db/local-name.last"
        ]
        && job.UserName or null == null
        && job.WatchPaths or [ ] == [ "/Library/Preferences/SystemConfiguration/preferences.plist" ]
        && job.StartInterval or null == 60
        && !(
          macNamed {
            networking.hostName = "a-mac";
            networking.localHostName = null;
          }
          ? local-name
        )
      );
}
# What follows runs the job's script under Linux's bash, with a stand-in
# scutil, since a Mac's sandbox can't set its own name.
// onLinux {
  # A renamed Mac's name is set back and the rename said once, at most once
  # in five minutes; a name already right is left unwritten, and one that
  # can't be read is left alone (tests/local-name-comes-back.sh).
  local-name-comes-back = pkgs.runCommandLocal "local-name-comes-back" { } ''
    ${lib.getExe pkgs.bash} ${./tests/local-name-comes-back.sh} ${./keep.sh} ${lib.getExe pkgs.bash}
    touch $out
  '';
}

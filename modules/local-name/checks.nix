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
  named = macNamed { networking.hostName = "a-mac"; };
  job = named.local-name.serviceConfig or { };
in
{
  # A Mac with a name runs, as root, the job that sets it back, with that
  # name, whenever macOS writes its network settings and once a minute; a Mac
  # with none runs nothing.
  local-name-is-kept-by-a-job =
    holds "local-name-is-kept-by-a-job"
      "A Mac's local name isn't kept by a job with its own name, watching its network settings, or a Mac with no name runs one."
      (
        lib.last (job.ProgramArguments or [ ]) == "a-mac"
        && job.WatchPaths or [ ] == [ "/Library/Preferences/SystemConfiguration/preferences.plist" ]
        && job.StartInterval or null == 60
        && !(macNamed { networking.localHostName = null; } ? local-name)
      );
}
# What follows runs the job's script under Linux's bash, with a stand-in
# scutil, since a Mac's sandbox can't set its own name.
// onLinux {
  # A renamed Mac's name is set back and the rename said once; a name
  # already right is left unwritten (tests/local-name-comes-back.sh).
  local-name-comes-back = pkgs.runCommandLocal "local-name-comes-back" { } ''
    ${lib.getExe pkgs.bash} ${./tests/local-name-comes-back.sh} ${./keep.sh} ${lib.getExe pkgs.bash}
    touch $out
  '';
}

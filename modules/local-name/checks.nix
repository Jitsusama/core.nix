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
  macLaunchd =
    settings:
    (bareDarwin [
      self.darwinModules.local-name
      settings
    ]).config.launchd;
  macNamed = settings: (macLaunchd settings).daemons;
  # A local name of its own, apart from the host name nix-darwin would
  # derive it from, so a job reading the host name instead is caught.
  itsOwnName = {
    networking.hostName = "a-mac";
    networking.localHostName = "its-own-name";
  };
  named = macNamed itsOwnName;
  job = named.local-name.serviceConfig or { };
  # Every launchd job the module adds, daemon or agent, beside a bare Mac's.
  # Only local-name may be one: any other could advertise for the host,
  # through dns-sd, a script of its own or launchd's own Bonjour key, and the
  # module's header says why none may. Held by name, so a new job fails
  # whatever it runs, and keep.sh, the one script it runs, is read for
  # dns-sd.
  added =
    let
      bare = (bareDarwin [ ]).config.launchd;
      given = macLaunchd itsOwnName;
      new = jobs: lib.subtractLists (lib.attrNames (jobs bare)) (lib.attrNames (jobs given));
    in
    new (l: l.daemons) ++ new (l: l.agents) ++ new (l: l.user.agents);
  mayAdvertise = added != [ "local-name" ] || lib.hasInfix "dns-sd" (builtins.readFile ./keep.sh);
in
{
  # A Mac with a local name runs, as root, keep.sh with that name and its
  # stamp, from its start, whenever macOS writes its network settings and
  # once a minute, under bash, finding scutil only in macOS's own folders
  # and saying what it does in its log; a Mac with a host name but no local
  # name runs nothing; and the module adds no other launchd job, nor does
  # keep.sh name dns-sd.
  local-name-is-kept-by-a-job =
    holds "local-name-is-kept-by-a-job"
      "A Mac's local name isn't kept by a root job running keep.sh under bash with its local name from its start, watching its network settings, finding its tools only in macOS's own folders and saying what it does in its log, or a Mac with no local name runs one, or the module adds another launchd job or keep.sh names dns-sd, either of which could advertise for the host."
      (
        !mayAdvertise
        && lib.hasSuffix "/bin/bash" (lib.head (job.ProgramArguments or [ "" ]))
        &&
          lib.drop 1 (job.ProgramArguments or [ ]) == [
            "${./keep.sh}"
            "its-own-name"
            "/var/db/local-name.last"
          ]
        && job.UserName or null == null
        && job.WatchPaths or [ ] == [ "/Library/Preferences/SystemConfiguration/preferences.plist" ]
        && job.StartInterval or null == 60
        && job.RunAtLoad or null == true
        && job.Disabled or null != true
        && job.EnvironmentVariables.PATH or null == "/usr/sbin:/usr/bin:/bin"
        && job.StandardErrorPath or null == "/var/log/local-name.log"
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
  # in five minutes by the time its stamp holds; a name already right is
  # left unwritten, one that can't be read is left alone, and what it
  # couldn't read or set is said at most once in five minutes
  # (tests/local-name-comes-back.sh).
  local-name-comes-back = pkgs.runCommandLocal "local-name-comes-back" { } ''
    ${lib.getExe pkgs.bash} ${./tests/local-name-comes-back.sh} ${./keep.sh} ${lib.getExe pkgs.bash}
    touch $out
  '';
}

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
  noName = {
    networking.hostName = "a-mac";
    networking.localHostName = null;
  };
  named = macNamed itsOwnName;
  job = named.local-name.serviceConfig or { };
  # Every launchd job a Mac's system writes, as the text of its plist, by
  # file name: nix-darwin lowers its launchd options into these, and takes
  # raw plists here too. Beside them, the agents home-manager gives the
  # Mac's one account.
  jobsOf =
    modules:
    let
      inherit (bareDarwin modules) config;
      plists = lib.mapAttrs (_: file: if file.text or null != null then file.text else "${file.source}");
    in
    {
      daemons = plists config.environment.launchDaemons;
      agents = plists config.environment.launchAgents;
      userAgents = plists config.environment.userLaunchAgents;
      accountAgents = lib.mapAttrs (_: agent: builtins.toJSON agent.config) (
        lib.filterAttrs (_: agent: agent.enable) config.home-manager.users.someone.launchd.agents
      );
    };
  # The jobs the module adds or changes on a Mac given these settings,
  # beside the same Mac without it, so a job added any of those ways, or a
  # key added to one the Mac already has, shows by name. Any of them could
  # advertise for the host, through dns-sd, a script of its own or launchd's
  # own Bonjour key, and the module's header says why none may.
  changedBy =
    settings:
    let
      before = jobsOf [ settings ];
      after = jobsOf [
        self.darwinModules.local-name
        settings
      ];
    in
    lib.concatLists (
      lib.mapAttrsToList (
        kind: files:
        lib.filter (file: before.${kind}.${file} or null != files.${file}) (lib.attrNames files)
      ) after
    );
  changed = changedBy itsOwnName ++ changedBy noName;
  # Only local-name's own plist, on a Mac with a local name; and that job
  # opens no socket, so launchd's Bonjour key can't ride on it, while its
  # arguments are held below and keep.sh, the one script it runs, is read
  # for dns-sd.
  mayAdvertise =
    changed != [ "${job.Label or "local-name"}.plist" ]
    || job.Sockets or null != null
    || lib.hasInfix "dns-sd" (builtins.readFile ./keep.sh);
in
{
  # A Mac with a local name runs, as root, keep.sh with that name and its
  # stamp, from its start, whenever macOS writes its network settings and
  # once a minute, under bash, finding scutil only in macOS's own folders
  # and saying what it does in its log; a Mac with a host name but no local
  # name runs nothing; and the module adds or changes no other launchd job,
  # nix-darwin's or the account's, that job opens no socket, and keep.sh
  # names no dns-sd.
  local-name-is-kept-by-a-job =
    holds "local-name-is-kept-by-a-job"
      "A Mac's local name isn't kept by a root job running keep.sh under bash with its local name from its start, watching its network settings, finding its tools only in macOS's own folders and saying what it does in its log, or a Mac with no local name runs one, or the module adds or changes another launchd job, opens a socket on its own or names dns-sd in keep.sh, any of which could advertise for the host. The jobs it adds or changes: ${lib.concatStringsSep ", " changed}."
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
        && !(macNamed noName ? local-name)
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

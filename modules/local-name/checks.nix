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
  # Each Mac evaluated once, with the module and without it, since an
  # evaluation is most of what a check costs.
  macWith = modules: (bareDarwin modules).config;
  namedSystem = bareDarwin [
    self.darwinModules.local-name
    itsOwnName
  ];
  namedMac = namedSystem.config;
  unnamedMac = macWith [
    self.darwinModules.local-name
    noName
  ];
  job = namedMac.launchd.daemons.local-name.serviceConfig or { };
  label = "${job.Label or "local-name"}.plist";
  # Every launchd job a Mac's system writes, as the file it links, by file
  # name: nix-darwin lowers its launchd options into these, and takes raw
  # plists here too, a forced source winning over the text. Beside them,
  # the agents home-manager gives the Mac's one account and any file it
  # puts in the account's LaunchAgents; whether Remote Login is turned on,
  # which writes no plist; and the activation scripts but nix-darwin's
  # launchd ones, whose text names every plist.
  jobsOf =
    config:
    let
      plists = lib.mapAttrs (
        _: file:
        builtins.toJSON {
          inherit (file) enable target;
          source = "${file.source}";
        }
      );
      account = config.home-manager.users.someone;
      launchAgentFiles = lib.filterAttrs (
        _: file: lib.hasPrefix "Library/LaunchAgents/" file.target
      ) account.home.file;
    in
    {
      daemons = plists config.environment.launchDaemons;
      agents = plists config.environment.launchAgents;
      userAgents = plists config.environment.userLaunchAgents;
      accountAgents = lib.mapAttrs (_: agent: builtins.toJSON agent.config) (
        lib.filterAttrs (_: agent: agent.enable) account.launchd.agents
      );
      accountFiles = lib.mapAttrs' (
        _: file:
        lib.nameValuePair file.target (
          builtins.toJSON [
            file.enable
            "${file.source}"
          ]
        )
      ) launchAgentFiles;
      remoteLogin."Remote Login" = builtins.toJSON (config.services.openssh.enable or null);
      activation = lib.mapAttrs (_: script: script.text) (
        removeAttrs config.system.activationScripts [
          "launchd"
          "userLaunchd"
          "script"
        ]
      );
    };
  # What the module adds or changes on a Mac, beside the same Mac without
  # it, so a job added any of those ways, or a key added to one the Mac
  # already has, shows by name. Any of them could advertise for the host,
  # through dns-sd, a script of its own, launchd's own Bonjour key or
  # Remote Login, and the module's header says why none may.
  changedBy =
    settings: mac:
    let
      before = jobsOf (macWith [ settings ]);
      after = jobsOf mac;
    in
    lib.concatLists (
      lib.mapAttrsToList (
        kind: files:
        lib.filter (file: before.${kind}.${file} or null != files.${file}) (lib.attrNames files)
      ) after
    );
  changedNamed = changedBy itsOwnName namedMac;
  changedUnnamed = changedBy noName unnamedMac;
  # The plist local-name's job writes, as written, not as its options say,
  # beside the file its text makes, with the Mac's own packages as
  # nix-darwin makes it.
  written = namedMac.environment.launchDaemons.${label} or { };
  fromText = namedSystem.pkgs.writeText label written.text or "";
  opensSocket =
    job.Sockets or null != null
    || lib.hasInfix "Sockets" written.text or ""
    || lib.hasInfix "Bonjour" written.text or ""
    || "${written.source or ""}" != "${fromText}";
  namesDnsSd = lib.hasInfix "dns-sd" (builtins.readFile ./keep.sh);
  yesNo = holding: if holding then "yes" else "no";
  # Only local-name's own plist, on a Mac with a local name, and nothing on
  # one without; that plist opens no socket, so launchd's Bonjour key can't
  # ride on it, while its arguments are held below and keep.sh, the one
  # script it runs, is read for dns-sd.
  mayAdvertise = changedNamed != [ label ] || changedUnnamed != [ ] || opensSocket || namesDnsSd;
in
{
  # A Mac with a local name runs, as root, keep.sh with that name and its
  # stamp, from its start, whenever macOS writes its network settings and
  # once a minute, under bash, finding scutil only in macOS's own folders
  # and saying what it does in its log; a Mac with a host name but no local
  # name runs nothing; and the module adds or changes no other launchd job,
  # nix-darwin's or the account's, nor Remote Login or an activation script,
  # the plist that job writes opens no socket, and keep.sh names no dns-sd.
  # It doesn't see another module's jobs, a deliberately disguised spelling
  # in keep.sh, or a line added to nix-darwin's own launchd activation or to
  # home-manager's.
  local-name-is-kept-by-a-job =
    holds "local-name-is-kept-by-a-job"
      "A Mac's local name isn't kept by a root job running keep.sh under bash with its local name from its start, watching its network settings, finding its tools only in macOS's own folders and saying what it does in its log, or a Mac with no local name runs one, or the module adds or changes another launchd job, Remote Login or an activation script, opens a socket on its own or names dns-sd in keep.sh, any of which could advertise for the host. What it adds or changes with a local name: ${lib.concatStringsSep ", " changedNamed}; without one: ${lib.concatStringsSep ", " changedUnnamed}. Its plist opens a socket: ${yesNo opensSocket}. keep.sh names dns-sd: ${yesNo namesDnsSd}."
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

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
  macWith =
    settings:
    (bareDarwin [
      self.darwinModules.local-name
      settings
    ]).config;
  macNamed = settings: (macWith settings).launchd.daemons;
  # A local name of its own, apart from the host name nix-darwin would
  # derive it from, so a job reading the host name instead is caught.
  itsOwnName = {
    networking.hostName = "a-mac";
    networking.localHostName = "its-own-name";
  };
  named = macNamed itsOwnName;
  # newsyslog's lines for the two logs, each split into its fields.
  rotation =
    let
      lines = lib.splitString "\n" (
        (macWith itsOwnName).environment.etc."newsyslog.d/local-name.conf".text or ""
      );
    in
    map (line: lib.filter (field: field != "") (lib.splitString " " line)) (
      lib.filter (line: line != "" && !(lib.hasPrefix "#" line)) lines
    );
  job = named.local-name.serviceConfig or { };
  answers = named.local-name-answers.serviceConfig or { };
in
{
  # A Mac with a local name runs, as root, keep.sh with that name and its
  # stamp, from its start, whenever macOS writes its network settings and
  # once a minute, under bash, finding scutil only in macOS's own folders
  # and saying what it does in its log; a Mac with a host name but no local
  # name runs nothing.
  local-name-is-kept-by-a-job =
    holds "local-name-is-kept-by-a-job"
      "A Mac's local name isn't kept by a root job running keep.sh under bash with its local name from its start, watching its network settings, finding its tools only in macOS's own folders and saying what it does in its log, or a Mac with no local name runs one."
      (
        lib.hasSuffix "/bin/bash" (lib.head (job.ProgramArguments or [ "" ]))
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

  # A Mac with a local name advertises itself as a workstation by that name,
  # with macOS's own dns-sd, as root, from its start and again whenever it
  # ends, saying what dns-sd says in its log; a Mac with no local name
  # advertises nothing.
  local-name-answers-by-a-job =
    holds "local-name-answers-by-a-job"
      "A Mac's local name isn't advertised by a root job running macOS's dns-sd as a workstation by that name from its start and again whenever it ends, saying what dns-sd says in its log, or a Mac with no local name advertises one."
      (
        answers.ProgramArguments or [ ] == [
          "/usr/bin/dns-sd"
          "-R"
          "its-own-name"
          "_workstation._tcp"
          "local"
          "9"
        ]
        && answers.UserName or null == null
        && answers.RunAtLoad or null == true
        && answers.KeepAlive or null == true
        && answers.StandardOutPath or null == "/var/log/local-name-answers.log"
        && answers.StandardErrorPath or null == "/var/log/local-name-answers.log"
        && answers.Disabled or null != true
        && !(
          macNamed {
            networking.hostName = "a-mac";
            networking.localHostName = null;
          }
          ? local-name-answers
        )
      );

  # A Mac with a local name has newsyslog keep each of the two logs under
  # 512 KB, with three before it, sending no signal; a Mac with no local
  # name adds nothing to newsyslog.
  local-name-logs-are-rotated =
    holds "local-name-logs-are-rotated"
      "A Mac's local-name logs aren't kept by newsyslog under 512 KB with three before each and no signal sent, or a Mac with no local name has newsyslog lines for them."
      (
        rotation == [
          [
            "/var/log/local-name.log"
            "644"
            "3"
            "512"
            "*"
            "N"
          ]
          [
            "/var/log/local-name-answers.log"
            "644"
            "3"
            "512"
            "*"
            "N"
          ]
        ]
        && !(
          (macWith {
            networking.hostName = "a-mac";
            networking.localHostName = null;
          }).environment.etc
            ? "newsyslog.d/local-name.conf"
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

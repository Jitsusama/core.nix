# Runs house-report on a Mac that shares a line with a shaped machine, as
# a launchd daemon (system.nix says what it's told). macOS lets a daemon
# reach the local network unasked, but blocks an account's own agent there
# until someone allows it, and asks no one for a program Nix built, so an
# agent would send nothing and say nothing of why. It runs as nobody, since
# it needs only a counter any account may read and a socket to send from.
{ config, lib, ... }:
let
  log = "/var/log/house-report.log";
  program = lib.escapeShellArgs config.jitsusama.house-report.arguments;
  inherit (config.jitsusama.house-report) keyFile;
  # Where activation copies the key for the daemon, in a folder only it
  # writes.
  copied = "/var/db/house-report";
  copy = "${copied}/house.key";
  # The trees whose folders a Mac makes root's alone, with no other
  # account able to rename one: the key is kept under one, spelled
  # plainly.
  rooted = [
    "/etc/"
    "/private/etc/"
    "/var/root/"
  ];
  plain =
    path:
    lib.any (tree: lib.hasPrefix tree path) rooted
    && !lib.any (part: lib.hasInfix part path) [
      "//"
      "/./"
      "/../"
    ];
  # Copies the key, readable by nobody's account alone, where the daemon
  # reads it each second, or takes the copy away and says why; activation runs it, and
  # so does house-report-key whenever the key or its folder changes, so
  # a key put in place or swapped is taken within seconds, as Linux's
  # path unit takes one there. Copied, the key itself stays root's, and
  # no second name for it, whoever made one, can keep it from the daemon
  # or hand another file over. The folders tested are the key's real
  # ones, every link resolved, each up to / root's and written by no
  # other account, by its mode or an access list; the key is then read
  # in the real folder tested, not by its spelling, whose links may sit
  # in folders never tested, so none but root may rename one between the
  # test and the copy, nor swap the key for a link after it's refused one
  # (house signing round 3's F1, F8, F12 and F13; round 4's F1, F8, F16,
  # F20 and F24). A copy that fails leaves the daemon no key, said, and
  # ends nothing, a switch included, its cleanup too (round 3's F10,
  # round 4's F14). Each run copies through a name of its own, so
  # activation and the watch job running at once can't take away the
  # copy the other made (round 4's F21). The Mac's own tools are named,
  # since GNU's stat, which a PATH may name first, reads -f otherwise.
  # nobody is an account other daemons may share, so any of them could
  # read the copy (round 4's F26).
  #
  # Kept as text here rather than a script of its own, which a sandboxed
  # check could walk over a tree of folders, until that follows (round
  # 4's F7).
  copying = ''
    houseReportKey=${lib.escapeShellArg keyFile}
    houseReportCopy=${copy}
    houseReportWhy=
    /usr/bin/install -d -o root -g wheel -m 0755 ${copied} \
      || houseReportWhy="has nowhere to be copied to"
    houseReportFolder=$(cd -P "$(/usr/bin/dirname "$houseReportKey")" 2>/dev/null && pwd -P) \
      || houseReportWhy="is in a folder that isn't there"
    houseReportRead="$houseReportFolder/$(/usr/bin/basename "$houseReportKey")"
    houseReportNew="$houseReportCopy.$$"
    while [ -z "$houseReportWhy" ] && [ "$houseReportFolder" != / ]; do
      if [ "$(/usr/bin/stat -f %u "$houseReportFolder")" != 0 ]; then
        houseReportWhy="is under $houseReportFolder, which isn't root's"
      elif [ -n "$(/usr/bin/find "$houseReportFolder" -maxdepth 0 -perm +022)" ]; then
        houseReportWhy="is under $houseReportFolder, which another account may write"
      elif /bin/ls -lde "$houseReportFolder" \
        | /usr/bin/grep -qE 'allow.*(add_file|add_subdirectory|delete_child|write)'; then
        houseReportWhy="is under $houseReportFolder, whose access list lets an account write"
      fi
      houseReportFolder=$(/usr/bin/dirname "$houseReportFolder")
    done
    if [ -n "$houseReportWhy" ]; then
      :
    elif [ -L "$houseReportRead" ]; then
      houseReportWhy="is a link"
    elif [ ! -f "$houseReportRead" ]; then
      houseReportWhy="isn't there as a file"
    elif ! /usr/bin/install -o nobody -g nobody -m 0400 "$houseReportRead" "$houseReportNew" \
      || ! /bin/mv -f "$houseReportNew" "$houseReportCopy"; then
      houseReportWhy="couldn't be copied for nobody"
    fi
    if [ -n "$houseReportWhy" ]; then
      /bin/rm -f "$houseReportCopy" "$houseReportNew" 2>/dev/null || :
      echo "house-report: $houseReportKey $houseReportWhy, so the daemon has no key" >&2
    fi
  '';
in
{
  imports = [ ./system.nix ];

  assertions = lib.optional (keyFile != null) {
    assertion = plain (toString keyFile);
    message =
      "jitsusama.house-report.keyFile: on a Mac the key must be under /etc, /private/etc "
      + "or /var/root, spelled plainly, where no other account may rename a folder above "
      + "it: ${toString keyFile}";
  };

  jitsusama.house-report.keyPath = lib.mkIf (keyFile != null) copy;

  # Made before launchd starts the daemon, and nobody's, so the daemon can
  # write its log whether launchd opens it before or after it takes on
  # nobody's account.
  system.activationScripts.preActivation.text = lib.mkAfter (
    ''
      touch ${log}
      chown nobody:nobody ${log}
      chmod 0644 ${log}
    ''
    # The daemon reads its key as nobody, so the key in place as the
    # machine switches is copied for it (`copying`); until one is there,
    # it reports nothing and says so.
    + lib.optionalString (keyFile != null) copying
  );

  # Copies the key again whenever it or its folder changes, as root, so a
  # key swapped while the Mac runs is taken without a switch, and says in
  # the daemon's log why one can't be.
  launchd.daemons.house-report-key.serviceConfig = lib.mkIf (keyFile != null) {
    ProgramArguments = [
      "/bin/sh"
      "-c"
      copying
    ];
    WatchPaths = [
      (toString keyFile)
      (dirOf (toString keyFile))
    ];
    RunAtLoad = true;
    StandardErrorPath = log;
  };

  launchd.daemons.house-report.serviceConfig = {
    UserName = "nobody";
    GroupName = "nobody";
    # At boot the program, a link into the store, isn't there until the
    # store's volume is mounted, so sh waits for the store first, as
    # nix-darwin's own daemons do.
    ProgramArguments = [
      "/bin/sh"
      "-c"
      "/bin/wait4path /nix/store && exec ${program}"
    ];
    RunAtLoad = true;
    # launchd waits 10 s between starts should it end.
    KeepAlive = true;
    # Not Background: macOS would mark its socket's traffic background
    # too, and a report held behind the upload it reports on comes late.
    ProcessType = "Standard";
    # Only troubles and their ends, each once, so it grows a line a change.
    StandardErrorPath = log;
  };
}

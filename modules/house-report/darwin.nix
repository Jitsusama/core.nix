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
    ]
    # Nor ending in a dot, as the shaper's own test refuses (house signing
    # round 5's F4).
    && !lib.any (end: lib.hasSuffix end path) [
      "/."
      "/.."
    ];
  # Copies the key, readable by nobody's account alone, where the daemon
  # reads it each second, or takes the copy away and says why; activation runs it, and
  # so does house-report-key whenever the key or its folder changes, so
  # a key put in place or swapped is taken within seconds, as Linux's
  # path unit takes one there. Copied, the key itself stays root's, and
  # no second name for it, whoever made one, can keep it from the daemon
  # or hand another file over. The folders tested are the key's real
  # ones, every link resolved, each up to / root's and written or taken
  # by no other account, by its mode or an access list, the key root's
  # and read by no other account itself; the key is then read
  # in the real folder tested, not by its spelling, whose links may sit
  # in folders never tested, so none but root may rename one between the
  # test and the copy, nor swap the key for a link after it's refused one
  # (house signing round 3's F1, F8, F12 and F13; round 4's F1, F8, F16,
  # F20 and F24; round 5's F1, F8, F14 and F15). A copy that fails
  # leaves the daemon no key, said once until the reason changes, and
  # ends nothing, a switch included, its cleanup too (round 3's F10,
  # round 4's F14), which takes away any run's leftover name too (round
  # 5's F9). A key another account owns, may read or write, or may read,
  # write, take or change the rights of by an access list, is refused
  # saying, after what it costs, to make it anew on both machines, since
  # that account may know it or have changed it, and a new owner or mode
  # leaves it so; one other accounts may only run, or reach by lesser
  # rights, is refused without it (keys said plainly round 1's F2, F3,
  # F10 and F14; round 2's F6 and F9; round 3's F8 and F11). Only the
  # first refusal that holds is said, so whether to say it is decided
  # apart from them, by the same three tests of the file the key's name
  # comes to, every link followed, and by its names, whatever refusal is
  # said: one of its folder, its link, its copy's folder or one of the
  # key's own (keys said plainly round 2's F1, F8 and F10; round 3's F1,
  # F5, F6, F7 and F10). Those tests copy nothing, so a folder another
  # account may change can only make the advice wrong, never hand a key
  # over. A folder's refusal, which says the key "is under" it, says to
  # make the key anew too, since an account that may change the folder
  # may have put another of root's files at the key's name by a hard
  # link, which outlasts the folder's fix; and a key with a second name
  # is refused, saying the same (keys said plainly round 6's F4).
  # Following every link takes readlink's -f, which macOS has from 12.3
  # (round 6's F1, F2, F5 and F6). Each
  # run copies through a name of its own, so two runs
  # can't move the same file (round 4's F21), though a failing run may
  # take away the copy one beside it just made, or a slower one put back
  # a key just swapped; the watch job runs again each minute too, so
  # either, or an event launchd missed, lasts a minute at most (round
  # 5's F2, F11, F12 and F16). The Mac's own tools are named,
  # since GNU's stat, which a PATH may name first, reads -f otherwise.
  # nobody is an account other daemons may share, so any of them could
  # read the copy (round 4's F26). A switch, which sets
  # houseReportAtSwitch, says a refusal on its own terminal each time and
  # forgets `said`; only the watch job, whose words go to the log, says it
  # once until it changes, so a refusal met at a switch reaches the log
  # within a minute, even one the log was cleared of since (house keys
  # round 1's F1, F6, F13 and F17; round 2's F9). `said` sits beside the
  # copy's folder, not in it, so a folder that can't be made is said once
  # too (round 2's F2), and the one once kept in it goes, so a rollback
  # reads no stale refusal there (made anew round 1's F3 and F8).
  #
  # Kept as text here rather than a script of its own, which a sandboxed
  # check could walk over a tree of folders, until that follows (round
  # 4's F7).
  copying = ''
    houseReportKey=${lib.escapeShellArg keyFile}
    houseReportCopy=${copy}
    houseReportWhy=
    houseReportDo=
    houseReportKnown="; make it anew here and on the shaped machine,"
    houseReportKnown="$houseReportKnown as another account may know or have changed it"
    houseReportKnowing='^ *[0-9]+: .* allow (.*,)?(read|write|append|writesecurity|chown)(,|$)'
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
        | /usr/bin/grep -qE 'allow.*(add_file|add_subdirectory|chown|delete|write)'; then
        houseReportWhy="is under $houseReportFolder, whose access list lets an account change it"
      fi
      houseReportFolder=$(/usr/bin/dirname "$houseReportFolder")
    done
    if [ -n "$houseReportWhy" ]; then
      :
    elif [ -L "$houseReportRead" ]; then
      houseReportWhy="is a link"
    elif [ ! -f "$houseReportRead" ]; then
      houseReportWhy="isn't there as a file"
    elif [ "$(/usr/bin/stat -f %l "$houseReportRead")" != 1 ]; then
      houseReportWhy="has another name, which another account may have given it"
    elif [ "$(/usr/bin/stat -f %u "$houseReportRead")" != 0 ]; then
      houseReportWhy="isn't root's"
    elif [ -n "$(/usr/bin/find "$houseReportRead" -maxdepth 0 -perm +066)" ]; then
      houseReportWhy="may be read or written by an account other than root's"
    elif /bin/ls -le "$houseReportRead" \
      | /usr/bin/grep -qE "$houseReportKnowing"; then
      houseReportWhy="has an access list letting others read, write or take it or change its rights"
    elif [ -n "$(/usr/bin/find "$houseReportRead" -maxdepth 0 -perm +011)" ]; then
      houseReportWhy="may be run by an account other than root's"
    elif /bin/ls -le "$houseReportRead" | /usr/bin/grep -qE '^ *[0-9]+: .* allow '; then
      houseReportWhy="has an access list letting another account in"
    elif ! /usr/bin/install -o nobody -g nobody -m 0400 "$houseReportRead" "$houseReportNew" \
      || ! /bin/mv -f "$houseReportNew" "$houseReportCopy"; then
      houseReportWhy="couldn't be copied for nobody"
    fi
    houseReportSeen=$(/usr/bin/readlink -f "$houseReportRead" 2>/dev/null) || houseReportSeen=
    if [ -z "$houseReportWhy" ]; then
      :
    elif [ "''${houseReportWhy#is under }" != "$houseReportWhy" ]; then
      houseReportDo=$houseReportKnown
    elif [ -z "$houseReportFolder" ] \
      || [ ! -f "$houseReportSeen" ] || [ -L "$houseReportSeen" ]; then
      :
    elif [ "$(/usr/bin/stat -f %u "$houseReportSeen")" != 0 ] \
      || [ -n "$(/usr/bin/find "$houseReportSeen" -maxdepth 0 -perm +066)" ] \
      || /bin/ls -le "$houseReportSeen" | /usr/bin/grep -qE "$houseReportKnowing" \
      || [ "$(/usr/bin/stat -f %l "$houseReportSeen")" != 1 ]; then
      houseReportDo=$houseReportKnown
    fi
    houseReportSaid=${copied}.said
    /bin/rm -f ${copied}/said 2>/dev/null || :
    if [ -n "$houseReportWhy" ]; then
      /bin/rm -f "$houseReportCopy" "$houseReportCopy".* 2>/dev/null || :
      houseReportWhy="house-report: $houseReportKey $houseReportWhy, so the daemon has no key"
      houseReportWhy="$houseReportWhy$houseReportDo"
      if [ -n "''${houseReportAtSwitch:-}" ]; then
        echo "$houseReportWhy" >&2
        /bin/rm -f "$houseReportSaid" 2>/dev/null || :
      elif [ "$(/bin/cat "$houseReportSaid" 2>/dev/null)" != "$houseReportWhy" ]; then
        echo "$houseReportWhy" >&2
        echo "$houseReportWhy" 2>/dev/null >"$houseReportSaid" || :
      fi
    elif [ -z "''${houseReportAtSwitch:-}" ]; then
      /bin/rm -f "$houseReportSaid" 2>/dev/null || :
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
    + lib.optionalString (keyFile != null) ("houseReportAtSwitch=1\n" + copying)
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
    # Each minute too, so a change launchd missed, or two runs racing,
    # leaves a stale or missing copy a minute at most (house signing
    # round 5's F11); a run with nothing new says nothing.
    StartInterval = 60;
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

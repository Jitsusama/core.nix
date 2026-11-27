# The checks of house-report, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  bareDarwin,
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
let
  told = {
    jitsusama.house-report.to = "192.0.2.3";
  };
  mac =
    (bareDarwin [
      self.darwinModules.house-report
      told
    ]).config;
  job = mac.launchd.daemons.house-report.serviceConfig or { };
  program = lib.last (job.ProgramArguments or [ "" ]);
  prepared = mac.system.activationScripts.preActivation.text or "";
  linux =
    (bareNixos [
      self.nixosModules.house-report
      told
    ]).config.systemd.services.house-report.serviceConfig or { };
  # What a machine told `to` runs, and whether it builds, as a module
  # setting it would.
  started =
    to:
    (bareNixos [
      self.nixosModules.house-report
      { jitsusama.house-report.to = to; }
    ]).config.systemd.services.house-report.serviceConfig.ExecStart;
  takes = to: (builtins.tryEval (builtins.deepSeq (started to) true)).success;
  taken = [
    "192.0.2.3"
    "10.0.0.5"
  ];
  refused = [
    "the mac"
    "192.0.2.256"
    "1::2::3"
    "12345::1"
    "::ffff:c000:203"
    ":"
    "2001:db8::3"
    "fd00::3"
    "fe80::3"
  ];
  misread = lib.filter (to: !takes to) taken ++ lib.filter takes refused;
  # A Mac and a Linux machine given a key.
  keyed = lib.recursiveUpdate told { jitsusama.house-report.keyFile = "/etc/house.key"; };
  keyedMac =
    (bareDarwin [
      self.darwinModules.house-report
      keyed
    ]).config;
  keyedProgram = lib.last (
    keyedMac.launchd.daemons.house-report.serviceConfig.ProgramArguments or [ "" ]
  );
  keyedPrepared = keyedMac.system.activationScripts.preActivation.text or "";
  # Where `words` first come in the Mac's activation text, or past its end.
  placed =
    words:
    let
      parts = lib.splitString words keyedPrepared;
    in
    if lib.length parts < 2 then
      lib.stringLength keyedPrepared + 1
    else
      lib.stringLength (lib.head parts);
  # Every folder above the key is tested before the chain that copies
  # it, and the copy is that chain's last branch, so a refusal stops it.
  walked = placed "while [ -z \"$houseReportWhy\" ] && [ \"$houseReportFolder\" != / ]; do";
  # What the refusals of a key another account may know say to do,
  # making it anew rather than giving it a new owner or mode, and the
  # access list's rights that let an account know it.
  known =
    "houseReportKnown=\"; make it anew here and on the shaped machine,\"\n"
    + "houseReportKnown=\"$houseReportKnown as another account may know or have changed it\"";
  knowing =
    let
      rights = "(read|write|append|writesecurity|chown)";
    in
    "houseReportKnowing='^ *[0-9]+: .* allow (.*,)?${rights}(,|$)'";
  chained = lib.concatStringsSep "\n" [
    "if [ -n \"$houseReportWhy\" ]; then"
    "  :"
    "elif [ -L \"$houseReportRead\" ]; then"
    "  houseReportWhy=\"is a link\""
    "elif [ ! -f \"$houseReportRead\" ]; then"
    "  houseReportWhy=\"isn't there as a file\""
    "elif [ \"$(/usr/bin/stat -f %l \"$houseReportRead\")\" != 1 ]; then"
    "  houseReportWhy=\"has another name, which any account that can reach it may give it\""
    "elif [ \"$(/usr/bin/stat -f %u \"$houseReportRead\")\" != 0 ]; then"
    "  houseReportWhy=\"isn't root's\""
    "elif [ -n \"$(/usr/bin/find \"$houseReportRead\" -maxdepth 0 -perm +066)\" ]; then"
    "  houseReportWhy=\"may be read or written by an account other than root's\""
    "elif /bin/ls -le \"$houseReportRead\" \\"
    "  | /usr/bin/grep -qE \"$houseReportKnowing\"; then"
    (
      "  houseReportWhy=\"has an access list letting others read, write or take it "
      + "or change its rights\""
    )
    "elif [ -n \"$(/usr/bin/find \"$houseReportRead\" -maxdepth 0 -perm +011)\" ]; then"
    "  houseReportWhy=\"may be run by an account other than root's\""
    "elif /bin/ls -le \"$houseReportRead\" | /usr/bin/grep -qE '^ *[0-9]+: .* allow '; then"
    "  houseReportWhy=\"has an access list letting another account in\""
    (
      "elif ! /usr/bin/install -o nobody -g nobody -m 0400 \"$houseReportRead\" "
      + "\"$houseReportNew\" \\"
    )
  ];
  # Whether a refusal says to make the key anew, decided apart from
  # which refusal is said, as only the first that holds is: by the
  # owner, mode and access list of the file the key's name comes to,
  # every link followed, whatever refused it, so no order of the
  # refusals can lose it (made anew round 3's F6; keys said plainly
  # round 1's F3 and F14; round 2's F1, F8 and F10; round 3's F1, F5,
  # F6, F7 and F10), and by whether a folder of it, which an account may
  # change, refused it, since that account may have put another of
  # root's files at its name by a hard link (round 6's F4). A second
  # name alone says nothing of it, since any account that can reach a
  # file may give it one (round 7's F1); `known` pins the words.
  seen = "/usr/bin/readlink -f \"$houseReportRead\" 2>/dev/null";
  advised = lib.concatStringsSep "\n" [
    "houseReportSeen=$(${seen}) || houseReportSeen="
    "if [ -z \"$houseReportWhy\" ]; then"
    "  :"
    "elif [ \"\${houseReportWhy#is under }\" != \"$houseReportWhy\" ]; then"
    "  houseReportDo=$houseReportKnown"
    "elif [ -z \"$houseReportFolder\" ] \\"
    "  || [ ! -f \"$houseReportSeen\" ] || [ -L \"$houseReportSeen\" ]; then"
    "  :"
    "elif [ \"$(/usr/bin/stat -f %u \"$houseReportSeen\")\" != 0 ] \\"
    "  || [ -n \"$(/usr/bin/find \"$houseReportSeen\" -maxdepth 0 -perm +066)\" ] \\"
    "  || /bin/ls -le \"$houseReportSeen\" | /usr/bin/grep -qE \"$houseReportKnowing\"; then"
    "  houseReportDo=$houseReportKnown"
    "fi"
  ];
  # The job's own lines deciding the advice, as its text sets them, with
  # the words they join, its Mac tools renamed to the stand-ins
  # advice-test.sh gives them; with none, should the job hold them other
  # than once, so the test advises nothing and fails (keys said plainly
  # round 4's F1, F2 and F7).
  advisedRun =
    let
      words = lib.filter (
        line: lib.hasPrefix "houseReportKnown=" line || lib.hasPrefix "houseReportKnowing=" line
      ) (lib.splitString "\n" watched);
      parts = lib.splitString "\nhouseReportSeen=" watched;
      lines =
        if lib.length parts == 2 then
          "houseReportSeen=" + lib.head (lib.splitString "\nfi\n" (lib.last parts)) + "\nfi\n"
        else
          "";
    in
    lib.replaceStrings
      [ "/usr/bin/readlink " "/usr/bin/stat " "/usr/bin/find " "/bin/ls " "/usr/bin/grep " ]
      [ "stand_in_readlink " "stand_in_stat " "stand_in_find " "stand_in_ls " "grep " ]
      (lib.concatLines words + lines);
  # What a refusal does, whole: a switch says it each time and forgets
  # `said`, kept beside the copy's folder; the job says it once until it
  # changes, and forgets it once a copy is made; and the `said` kept in
  # the folder before goes, so a rollback can't read it (house keys
  # round 2's F1, F2, F4 and F9; made anew round 1's F3 and F8).
  noKey = "house-report: $houseReportKey $houseReportWhy, so the daemon has no key";
  refusing = lib.concatStringsSep "\n" [
    "houseReportSaid=/var/db/house-report.said"
    "/bin/rm -f /var/db/house-report/said 2>/dev/null || :"
    "if [ -n \"$houseReportWhy\" ]; then"
    "  /bin/rm -f \"$houseReportCopy\" \"$houseReportCopy\".* 2>/dev/null || :"
    "  houseReportWhy=\"${noKey}\""
    "  houseReportWhy=\"$houseReportWhy$houseReportDo\""
    "  if [ -n \"\${houseReportAtSwitch:-}\" ]; then"
    "    echo \"$houseReportWhy\" >&2"
    "    /bin/rm -f \"$houseReportSaid\" 2>/dev/null || :"
    "  elif [ \"$(/bin/cat \"$houseReportSaid\" 2>/dev/null)\" != \"$houseReportWhy\" ]; then"
    "    echo \"$houseReportWhy\" >&2"
    "    echo \"$houseReportWhy\" 2>/dev/null >\"$houseReportSaid\" || :"
    "  fi"
    "elif [ -z \"\${houseReportAtSwitch:-}\" ]; then"
    "  /bin/rm -f \"$houseReportSaid\" 2>/dev/null || :"
    "fi"
  ];
  # Each test the walk makes of a folder, whole, refusing as it says:
  # its owner, its mode and its access list's rights (round 5's F3 and
  # F6), each saying the key "is under" it, which `advised` reads.
  folderTests = [
    (lib.concatStringsSep "\n" [
      "if [ \"$(/usr/bin/stat -f %u \"$houseReportFolder\")\" != 0 ]; then"
      "    houseReportWhy=\"is under $houseReportFolder, which isn't root's\""
    ])
    (lib.concatStringsSep "\n" [
      "elif [ -n \"$(/usr/bin/find \"$houseReportFolder\" -maxdepth 0 -perm +022)\" ]; then"
      "    houseReportWhy=\"is under $houseReportFolder, which another account may write\""
    ])
    (lib.concatStringsSep "\n" [
      "elif /bin/ls -lde \"$houseReportFolder\" \\"
      "    | /usr/bin/grep -qE 'allow.*(add_file|add_subdirectory|chown|delete|write)'; then"
      (
        "    houseReportWhy=\"is under $houseReportFolder, "
        + "whose access list lets an account change it\""
      )
    ])
  ];
  # Each folder test is whole and inside the walk, before the chain.
  walksWhole = lib.all (
    test: lib.hasInfix test keyedPrepared && walked < placed test && placed test < placed chained
  ) folderTests;
  # The key is read in the real folder the walk tests, not by its
  # spelling.
  readInRealFolder =
    "houseReportRead=\"$houseReportFolder/" + "$(/usr/bin/basename \"$houseReportKey\")\"";
  # How many times `words` come in `text`.
  times = words: text: lib.length (lib.splitString words text) - 1;
  # Where a Mac's daemon reads activation's copy of its key.
  copied = "/var/db/house-report/house.key";
  watching = keyedMac.launchd.daemons.house-report-key.serviceConfig or { };
  watched = lib.last (watching.ProgramArguments or [ "" ]);
  # The pattern the job tests a key's access list by, as its text sets
  # it, so what it's run over below judges the job's own.
  knowingPattern = lib.removeSuffix "'" (
    lib.removePrefix "houseReportKnowing='" (
      lib.findFirst (lib.hasPrefix "houseReportKnowing='") "" (lib.splitString "\n" watched)
    )
  );
  # Access-list lines as a Mac's ls -le prints them, each entry on a
  # numbered line of its own, chmod +a joining an account's grants of
  # one kind into one entry, its rights in a fixed order, read and write
  # first, as this Mac's printed them on 25 November: those letting
  # another account know or change a key, and those that don't. Each
  # right taken has a line where it's the only one taken, and the lines
  # put a taken right first, mid-list and last (keys said plainly round
  # 3's F2, F3 and F4).
  leakingLists = [
    " 1: group:everyone allow read,write,append"
    " 0: user:someone inherited allow chown"
    " 0: group:everyone allow delete,writesecurity"
    " 0: group:staff allow append"
    " 0: group:everyone allow write"
    " 3: group:everyone allow read,readattr,readextattr,readsecurity"
    " 0: group:everyone allow write,writeattr"
    " 0: group:everyone allow delete,append,readattr"
    " 1: user:someone allow write,delete,readattr,readextattr,writesecurity,chown"
  ];
  lesserLists = [
    " 0: group:everyone allow readattr,readextattr,readsecurity"
    " 0: group:everyone allow readattr,writeattr,readsecurity"
    " 0: group:everyone allow execute,writeattr"
    " 0: user:someone allow delete,writeextattr"
    " 0: group:everyone deny read"
  ];
  # Whether `text` names the job's `said` only where `refusing` does:
  # where the job's text sets, forgets and reads it (keys said plainly
  # round 2's F7).
  namesSaidOnlyInRefusing =
    text:
    times "houseReportSaid" text == 5
    && times "houseReportSaid=" text == 1
    && times "house-report.said" text == 1
    && times "house-report/said" text == 1;
  # Whether a Mac given `keyFile` is refused as it builds.
  macRefuses =
    keyFile:
    lib.any (said: !said.assertion)
      (bareDarwin [
        self.darwinModules.house-report
        (lib.recursiveUpdate told { jitsusama.house-report.keyFile = keyFile; })
      ]).config.assertions;
  macKeys = {
    taken = [
      "/etc/house.key"
      "/private/etc/house-report/house.key"
      "/var/root/house.key"
    ];
    refused = [
      "/Users/someone/house.key"
      "/usr/local/etc/house.key"
      "/etc/../tmp/house.key"
      "/etc//house.key"
      "/etc/./house.key"
      "/etc/house-report/.."
      "/etc/house.key/."
    ];
  };
  macMisread = lib.filter macRefuses macKeys.taken ++ lib.filter (k: !macRefuses k) macKeys.refused;
  keyedLinux =
    (bareNixos [
      self.nixosModules.house-report
      keyed
    ]).config.systemd;
  keyedStart = keyedLinux.services.house-report.serviceConfig;
in
{
  # A Mac runs house-report as a daemon, as nobody, from its start and
  # again whenever it ends, once the store is there, at a standard
  # priority, writing its log in /var/log, which is made nobody's before
  # launchd starts it; and the address it's told reaches it with the port
  # the shaper hears on.
  house-report-runs-as-a-macs-daemon =
    holds "house-report-runs-as-a-macs-daemon"
      (
        "A Mac doesn't run house-report as a daemon, as nobody, from its start and again whenever "
        + "it ends, once the store is there, at a standard priority, logging to "
        + "/var/log/house-report.log made nobody's before it starts, sending to 192.0.2.3:47110."
      )
      (
        job.UserName or null == "nobody"
        && job.GroupName or null == "nobody"
        &&
          lib.take 2 (job.ProgramArguments or [ ]) == [
            "/bin/sh"
            "-c"
          ]
        && lib.hasPrefix "/bin/wait4path /nix/store && exec " program
        && lib.hasSuffix "/bin/house-report 192.0.2.3:47110" program
        && job.RunAtLoad or null == true
        && job.KeepAlive or null == true
        && job.ProcessType or null == "Standard"
        && job.StandardErrorPath or null == "/var/log/house-report.log"
        && lib.hasInfix "touch /var/log/house-report.log" prepared
        && lib.hasInfix "chown nobody:nobody /var/log/house-report.log" prepared
      );

  # A Mac given a key reads activation's copy of it, nobody's, by its
  # path ahead of the address. Activation copies the key only once every
  # folder above it is root's and written or taken by no other account,
  # by mode or access list, reading it in its real folder as a plain
  # file root's alone, the copy the last branch of the chain that
  # refuses, through a name of the run's own, and changes no file's
  # owner or mode. A key another account owns, may read or write, or may
  # reach so by an access list, is refused saying to make it anew on both
  # machines. The build refuses a key outside root's own trees, or
  # spelled through dots.
  house-report-hands-a-mac-its-key-safely =
    holds "house-report-hands-a-mac-its-key-safely"
      (
        if macMisread != [ ] then
          "a Mac's key check misreads: ${lib.concatStringsSep ", " macMisread}"
        else
          "A Mac given /etc/house.key doesn't run house-report --key "
          + "/var/db/house-report/house.key 192.0.2.3:47110, or its activation copies the key "
          + "outside the chain that refuses a link, a key not root's alone and a folder "
          + "another account may write or take, reads it by its spelling rather than its "
          + "real folder, or hands a file over otherwise than by one install into a name of "
          + "its own, or refuses a key another account may know or have changed without "
          + "saying, last and once, to make it anew on both machines, whichever refusal is "
          + "said."
      )
      (
        macMisread == [ ]
        && lib.hasSuffix "/bin/house-report --key ${copied} 192.0.2.3:47110" keyedProgram
        && lib.hasInfix "houseReportKey=/etc/house.key" keyedPrepared
        && lib.hasInfix chained keyedPrepared
        && walked < placed chained
        # The advice is set once, in the job's own text, before the
        # chain, and taken only past the chain, once, whatever it refused,
        # so no branch of it sets the advice (keys said plainly round 1's
        # F1, F2, F5, F6, F13 and F19; round 3's F5), and is said last,
        # joined after what the refusal costs (round 2's F2, F5 and F11).
        && lib.hasInfix chained watched
        && lib.hasInfix advised watched
        && lib.hasInfix refusing watched
        && lib.hasInfix known watched
        && lib.hasInfix knowing watched
        && lib.hasInfix "houseReportWhy=\nhouseReportDo=\n" watched
        && placed known < placed chained
        && placed knowing < placed chained
        && placed "houseReportDo=\n" < placed chained
        && placed chained < placed advised
        && placed advised < placed refusing
        && times "houseReportKnown" keyedPrepared == 5
        && times "houseReportKnowing" keyedPrepared == 3
        && times "houseReportDo" keyedPrepared == 4
        && lib.hasInfix "&& pwd -P)" keyedPrepared
        && walksWhole
        && lib.hasInfix readInRealFolder keyedPrepared
        && walked > placed "houseReportRead="
        # Set once, so nothing reads the key by its spelling after the
        # walk (round 5's F7 and F17).
        && times "houseReportRead=" watched == 1
        && lib.hasInfix "/usr/bin/install -d -o root -g wheel -m 0755 /var/db/house-report " watched
        # The copy's text names no chown or chmod, and no install or mv
        # but the one install into this run's own name, moved onto the
        # copy (round 4's F6), however a command is spelled (round 5's
        # F7); the access list's right of the name stays a word in the
        # walk's pattern. A cp, a cat or a redirect isn't counted: a
        # text check of an unrun script proves no more (house keys round
        # 1's F3, F10 and F15).
        && times "chown " watched == 0
        && times "chmod " watched == 0
        && times "install " watched == 2
        && times "mv " watched == 1
        && lib.hasInfix "/bin/mv -f \"$houseReportNew\" \"$houseReportCopy\"" watched
        && lib.hasInfix "houseReportNew=\"$houseReportCopy.$$\"" watched
        # The failing run's cleanup is the line that must never end a
        # switch, so it's pinned whole (house keys round 1's F9).
        && lib.hasInfix (
          "/bin/rm -f \"$houseReportCopy\" \"$houseReportCopy\".*" + " 2>/dev/null || :"
        ) watched
      );

  # The pattern a Mac's job tests a key's access list by takes a line
  # granting another account read, write, append, writesecurity or chown,
  # alone or among other rights, as letting it know the key, and not one
  # granting lesser rights or denying any, run over lines as ls -le
  # prints them (keys said plainly round 2's F3 and F4). GNU grep runs it
  # here, on a Mac's builder too; the Mac's own /usr/bin/grep took and
  # left lines of each kind as here, over what its ls -le printed on 25
  # November. It uses only POSIX's extended syntax.
  house-report-knows-which-lists-leak-a-key =
    pkgs.runCommandLocal "house-report-knows-which-lists-leak-a-key"
      {
        pattern = knowingPattern;
        leaking = lib.concatLines leakingLists;
        lesser = lib.concatLines lesserLists;
        passAsFile = [
          "leaking"
          "lesser"
        ];
      }
      ''
        "$BASH" ${./lists-test.sh} "$pattern" "$leakingPath" "$lesserPath"
        touch "$out"
      '';

  # A Mac's key job, run over sample keys with its Mac tools stood in for,
  # says to make a key anew whichever refusal it says, by the owner, mode
  # and access list of the file the key's name comes to, and for any key
  # its folder refused, and says it of no key others may only run or
  # reach by lesser rights, of one refused only for a second name, of one
  # it copied, or where the name comes to no file and no folder refused
  # it (keys said plainly round 4's F1, F2 and F7; round 6's F4; round
  # 7's F1).
  house-report-advises-by-the-file-its-key-names =
    pkgs.runCommandLocal "house-report-advises-by-the-file-its-key-names"
      {
        advice = advisedRun;
        passAsFile = [ "advice" ];
      }
      ''
        "$BASH" ${./advice-test.sh} "$advicePath"
        touch "$out"
      '';

  # A Mac given a key copies it again, as root, whenever the key or its
  # folder changes, and each minute, by the same text activation runs,
  # so a key swapped while the Mac runs is taken without a switch, and a
  # change launchd missed within a minute.
  house-report-takes-up-a-macs-swapped-key =
    holds "house-report-takes-up-a-macs-swapped-key"
      (
        "A Mac given /etc/house.key doesn't copy it again, as root, by activation's own text, "
        + "whenever /etc/house.key or /etc changes and each minute, or says a refusal "
        + "otherwise than at each switch, forgetting what the job said and the one once "
        + "kept in the copy's folder, and once in the job's log until it changes."
      )
      (
        lib.take 2 (watching.ProgramArguments or [ ]) == [
          "/bin/sh"
          "-c"
        ]
        && lib.hasInfix chained watched
        && lib.hasInfix watched keyedPrepared
        # A switch says a refusal each time and forgets the job's `said`,
        # so the log still gets it (house keys round 1's F1, F6, F13 and
        # F17; round 2's F1, F4 and F9).
        && lib.hasInfix ("houseReportAtSwitch=1\n" + watched) keyedPrepared
        && !lib.hasInfix "houseReportAtSwitch=" watched
        && lib.hasInfix refusing watched
        # Nothing else reads or writes `said`, under its name, braced or
        # not, or spelled as its path, in the job or anywhere else in the
        # switch's preActivation, which holds the job's text (made anew
        # round 1's F5; round 2's F2, F4, F5, F7 and F10; round 3's F3;
        # keys said plainly round 1's F4, F8 and F9).
        && namesSaidOnlyInRefusing keyedPrepared
        && watching.UserName or null == null
        && watching.RunAtLoad or null == true
        && watching.StartInterval or null == 60
        &&
          watching.WatchPaths or [ ] == [
            "/etc/house.key"
            "/etc"
          ]
      );
}
# What follows needs Linux to build or to run, so a Mac leaves it out.
// onLinux {
  # house-report's tests run as it builds: a report reads as the shaper
  # parses it, the total it sends never goes back, and the way reports
  # go is said each time it moves.
  house-report-says-what-the-link-sent = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in house-report, its tests included.
  house-report-passes-clippy = passesClippy "house-report-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # A Linux machine runs house-report as a service of its own account,
  # writing nothing, started again when it ends but not when it was asked
  # wrongly; and it's told only an address, as the shaper's house reads
  # one, and IPv4 alone, so a name, a malformed address or an IPv6 one is
  # refused as it builds, and an address is sent to with the shaper's port.
  house-report-is-told-an-address =
    holds "house-report-is-told-an-address"
      (
        if misread != [ ] then
          "house-report's address check misreads: ${lib.concatStringsSep ", " misread}"
        else
          "house-report doesn't run on Linux as a service of its own account, writing nothing, "
          + "started again unless it was asked wrongly, sending to 10.0.0.5:47110."
      )
      (
        misread == [ ]
        && lib.hasInfix " 10.0.0.5:47110" (started "10.0.0.5")
        && linux.DynamicUser or null == true
        && linux.ProtectSystem or null == "strict"
        && linux.Restart or null == "always"
        && linux.RestartPreventExitStatus or null == "64"
      );

  # A Linux machine given a key hands it to house-report as a credential,
  # which it reads by --key, and restarts it whenever the key file
  # changes, leaving one someone stopped stopped.
  house-report-takes-up-a-swapped-key =
    holds "house-report-takes-up-a-swapped-key"
      (
        "A Linux machine given /etc/house.key doesn't hand it over as a credential read by --key, "
        + "doesn't try-restart house-report when the file changes, or limits their starts."
      )
      (
        keyedStart.LoadCredential or null == "house.key:/etc/house.key"
        && lib.hasInfix "--key %d/house.key " (keyedStart.ExecStart or "")
        && keyedLinux.paths.house-report-key.pathConfig.PathChanged or null == "/etc/house.key"
        && builtins.elem "multi-user.target" (keyedLinux.paths.house-report-key.wantedBy or [ ])
        && lib.hasSuffix "/bin/systemctl try-restart house-report.service" (
          keyedLinux.services.house-report-key.serviceConfig.ExecStart or ""
        )
        && keyedLinux.services.house-report.startLimitIntervalSec or null == 0
        && keyedLinux.services.house-report-key.startLimitIntervalSec or null == 0
      );
}

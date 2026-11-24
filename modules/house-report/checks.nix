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
  # What the two refusals of a key another account could read say to do,
  # making it anew rather than giving it a new mode.
  leaked =
    "houseReportLeaked=\"make it anew on both machines, "
    + "since a new mode leaves what leaked good\"";
  chained = lib.concatStringsSep "\n" [
    "if [ -n \"$houseReportWhy\" ]; then"
    "  :"
    "elif [ -L \"$houseReportRead\" ]; then"
    "  houseReportWhy=\"is a link\""
    "elif [ ! -f \"$houseReportRead\" ]; then"
    "  houseReportWhy=\"isn't there as a file\""
    "elif [ \"$(/usr/bin/stat -f %u \"$houseReportRead\")\" != 0 ]; then"
    "  houseReportWhy=\"isn't root's\""
    "elif [ -n \"$(/usr/bin/find \"$houseReportRead\" -maxdepth 0 -perm +077)\" ]; then"
    # A key another account could read is said to need making anew, since
    # a new mode would leave the leaked bytes good (keys said plainly
    # round 1's F6); `leaked` pins the words.
    "  houseReportWhy=\"may be read by an account other than root's: $houseReportLeaked\""
    "elif /bin/ls -le \"$houseReportRead\" | /usr/bin/grep -qE '^ *[0-9]+: .* allow '; then"
    "  houseReportWhy=\"has an access list letting another account in: $houseReportLeaked\""
    (
      "elif ! /usr/bin/install -o nobody -g nobody -m 0400 \"$houseReportRead\" "
      + "\"$houseReportNew\" \\"
    )
  ];
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
  # F6).
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
  # Whether `text` names the job's `said` only where the switch sets it
  # and the job forgets and reads it.
  saysSaidOnce =
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
  # owner or mode. The build refuses a key outside root's own trees, or
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
          + "its own, or lets a key another account could read be taken for a new mode."
      )
      (
        macMisread == [ ]
        && lib.hasSuffix "/bin/house-report --key ${copied} 192.0.2.3:47110" keyedProgram
        && lib.hasInfix "houseReportKey=/etc/house.key" keyedPrepared
        && lib.hasInfix chained keyedPrepared
        && walked < placed chained
        && lib.hasInfix leaked keyedPrepared
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
        # not, or spelled as its path, in the job or anywhere else the
        # switch runs (made anew round 1's F5; round 2's F2, F4, F5, F7
        # and F10; keys said plainly round 1's F3).
        && lib.all saysSaidOnce [
          watched
          keyedPrepared
        ]
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

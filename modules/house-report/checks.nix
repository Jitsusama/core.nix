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
  handedOver = placed "/usr/sbin/chown -h nobody:nobody \"$houseReportKey\"";
  refusedFirst = map placed [
    "[ -L \"$houseReportKey\" ]"
    "/usr/bin/stat -f %l \"$houseReportKey\""
    "/usr/bin/stat -L -f %u \"$houseReportFolder\""
    "/usr/bin/find -H \"$houseReportFolder\" -maxdepth 0 -perm +022"
  ];
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

  # A Mac given a key reads it by its path, ahead of the address, and its
  # activation hands the key to nobody only after refusing a link, a
  # second name and a folder not root's alone, never following a link.
  house-report-hands-a-mac-its-key-safely =
    holds "house-report-hands-a-mac-its-key-safely"
      (
        "A Mac given /etc/house.key doesn't run house-report --key /etc/house.key 192.0.2.3:47110, "
        + "or its activation hands the key over before refusing a link, a second name or a "
        + "folder another account may write, or follows a link as it does."
      )
      (
        lib.hasSuffix "/bin/house-report --key /etc/house.key 192.0.2.3:47110" keyedProgram
        && lib.hasInfix "houseReportKey=/etc/house.key" keyedPrepared
        && handedOver < lib.stringLength keyedPrepared
        && lib.all (at: at < handedOver) refusedFirst
        && lib.hasInfix "/bin/chmod -h 0400 \"$houseReportKey\"" keyedPrepared
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
        + "or doesn't try-restart house-report when the file changes."
      )
      (
        keyedStart.LoadCredential or null == "house.key:/etc/house.key"
        && lib.hasInfix "--key %d/house.key " keyedStart.ExecStart
        && keyedLinux.paths.house-report-key.pathConfig.PathChanged or null == "/etc/house.key"
        && builtins.elem "multi-user.target" keyedLinux.paths.house-report-key.wantedBy
        && lib.hasSuffix "/bin/systemctl try-restart house-report.service" (
          keyedLinux.services.house-report-key.serviceConfig.ExecStart or ""
        )
      );
}

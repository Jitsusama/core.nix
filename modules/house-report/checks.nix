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
    "2001:db8::3"
  ];
  refused = [
    "the mac"
    "192.0.2.256"
    "1::2::3"
    "12345::1"
    "::ffff:c000:203"
    ":"
    "fe80::3"
    "FEBF::3"
  ];
  misread = lib.filter (to: !takes to) taken ++ lib.filter takes refused;
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
  # one, so a name, a malformed address or a link-local one is refused as
  # it builds, and an IPv6 one is sent to in brackets before its port.
  house-report-is-told-an-address =
    holds "house-report-is-told-an-address"
      (
        if misread != [ ] then
          "house-report's address check misreads: ${lib.concatStringsSep ", " misread}"
        else
          "house-report doesn't run on Linux as a service of its own account, writing nothing, "
          + "started again unless it was asked wrongly, sending to an IPv6 address in brackets."
      )
      (
        misread == [ ]
        && lib.hasInfix "[2001:db8::3]:47110" (started "2001:db8::3")
        && linux.DynamicUser or null == true
        && linux.ProtectSystem or null == "strict"
        && linux.Restart or null == "always"
        && linux.RestartPreventExitStatus or null == "64"
      );
}

# The network in the same order as the CPU, memory and disk: a call, music
# and what Joel waits on before the work he started, both ways.
#
# The queue that fills when this machine sends isn't its own: it's the
# modem's or the router's, first come first served, so a call's packets
# wait there behind a build's upload. With a 1 GB download running, pings
# to 1.1.1.1 over Ethernet took 173 ms at the median and 288 ms at the 95th
# percentile, against 17 ms quiet; with an upload, 197 ms through WARP. The
# connection carried about 11 Mbit/s up. So on every link with a default
# route the shaper sends and receives a little slower than the connection
# can take, through
# CAKE, and the queue forms here instead, short and sorted. How fast that is
# differs on every network, and on Wi-Fi from minute to minute, so the
# shaper finds it from what pings to four distant resolvers do, cutting
# the moment they slow and climbing while they don't, and keeps what each
# network held, by its router's hardware address. A network never met
# starts at 10 Mbit/s up and 50 down and climbs within seconds.
#
# CAKE's four tins follow the slices: voice for names, time and the
# shaper's own pings; video for Tuple's daemon, an app's UDP other than
# 443, which is how a browser's call sends, and what an app sends Meet's
# media servers, which take a call on 443; bulk for the jobs, background
# and containers slices, the account's user.slice, Nix's builds, virtual
# machines and whatever this machine forwards for them; best effort for the
# rest, Spotify, Chrome, Slack, pi's own requests and what Joel waits on at
# a terminal among them. Bulk gets the whole connection while nothing else
# wants it, and a sixteenth while something does. What a link reaches
# without the router isn't slowed.
#
# A VPN leaves and arrives by the connection as one flow, whatever it
# carries, so CAKE there can't tell a name lookup inside it from a build's
# upload, or a model's reply from a git fetch. A VPN named in
# jitsusama.shaper.vpns gets a CAKE of its own each way, sorted by the same
# classes, at a little under the connection's rates, so its queues form
# inside it.
#
# Each time the shaper starts, the kernel warns "htb: cake qdisc 2: is
# non-work-conserving?", once for the uplink and once for its IFB, and
# never again until the queues are made afresh. It's expected: CAKE keeps
# its own rate, so when HTB asks it for a packet it holds back, it gives
# none, and HTB says so the first time any child does (sch_htb.c,
# qdisc_warn_nonwc in pkt_sched.h). HTB's classes never limit, so CAKE's
# rate is the only one that does.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  shaper = pkgs.callPackage ./package.nix { };
  callIsOn = pkgs.callPackage ../calls/package.nix { };
  # `shaper proof` loads the link with curl only while swayidle says Joel
  # is away, a-call-is-on says no call is on and pw-dump shows nothing
  # playing, and reads the default route with ip; systemd-run and
  # systemctl are the system's own. What else it waits on, such as the
  # calendar, the machine names in proofWaitsWhile.
  waitsWhile = config.jitsusama.shaper.proofWaitsWhile;
  waitsFlag = lib.optionalString (waitsWhile != [ ]) (
    "--set-default SHAPER_PROOF_WAITS_WHILE " + lib.escapeShellArg (builtins.toJSON waitsWhile)
  );
  proofTools = [
    pkgs.curl
    pkgs.iproute2
    pkgs.pipewire
    pkgs.swayidle
    callIsOn
  ];
  shaperCommand = pkgs.symlinkJoin {
    name = "shaper";
    paths = [ shaper ];
    nativeBuildInputs = [ pkgs.makeWrapper ];
    postBuild = ''
      wrapProgram $out/bin/shaper --suffix PATH : ${lib.makeBinPath proofTools} \
        ${serverFlag} ${waitsFlag}
    '';
  };
  speedServer = config.jitsusama.shaper.speedServer;
  serverFlag = lib.optionalString (
    speedServer != null
  ) "--set-default SHAPER_SPEED_SERVER ${lib.escapeShellArg speedServer}";
  vpns = config.jitsusama.shaper.vpns;
  oneWay = config.jitsusama.shaper.controller == "one-way";
in
{
  imports = [
    ../nftables/nixos.nix
    # The glossary's word is VPN; the old name keeps working, with a warning,
    # until the machines that set it move.
    (lib.mkRenamedOptionModule [ "jitsusama" "shaper" "tunnels" ] [ "jitsusama" "shaper" "vpns" ])
  ];

  options.jitsusama.shaper.vpns = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [ "wg0" ];
    description = ''
      The VPNs to shape inside, by their links' names: those that ride the
      connection as one flow and carry traffic this machine sends.
    '';
  };

  options.jitsusama.shaper.controller = lib.mkOption {
    type = lib.types.enum [
      "round-trip"
      "one-way"
    ];
    default = "round-trip";
    description = ''
      What sets the shaper's rates. `round-trip` is the controller it has
      had (src/rate.rs), from pings to distant resolvers. `one-way` is its
      rebuild (src/oneway.rs), from each direction's own delay as
      shaper-record measures it, the controller `shaper replay` scores.
    '';
  };

  options.jitsusama.shaper.speedServer = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "https://speed.example.net";
    description = ''
      Where `shaper proof` loads the link: a server that takes what is
      posted to `/__up` and answers `/__down?bytes=N` with N bytes. Left
      unset, the proof asks for `--server`.
    '';
  };

  options.jitsusama.shaper.proofWaitsWhile = lib.mkOption {
    type = lib.types.listOf (lib.types.nonEmptyListOf lib.types.str);
    default = [ ];
    example = [
      [
        "a-meeting-is-near"
        "/home/me/Documents/Calls"
      ]
    ];
    description = ''
      What else `shaper proof` waits on before it loads the link, past
      Joel being idle, no call and no audio: commands run as him, each
      succeeding while the link must carry no load, such as within ten
      minutes of a meeting, and exiting 1 while it may. Anything else
      counts as busy, since it couldn't tell. A call can ring before its
      microphone opens, and the calendar knows it first.
    '';
  };

  config.networking.nftables.tables.shaper = {
    family = "inet";
    content = builtins.readFile ./shaper.nft;
  };

  # The ruleset is checked as it builds, against a kernel built as a
  # library, which can't match a socket's cgroup. The rules that do are
  # left out of the copy it checks; the VM tests load them whole.
  config.networking.nftables.preCheckRuleset = ''
    sed -i '/cgroupv2 level [0-9]* @/d' ruleset.conf
  '';

  # The shaper makes the IFBs it needs, one per link; the module's own two
  # would only sit there.
  config.boot.extraModprobeConfig = "options ifb numifbs=0";

  # udev gives every new link systemd's default .link, whose name, address
  # and queue settings an IFB can't take, and says so in the journal each
  # time the shaper makes one. The IFBs are the shaper's alone, named by
  # it, so udev is told to set nothing on them.
  config.systemd.network.links."10-shaper-ifb" = {
    matchConfig.Driver = "ifb";
    linkConfig.MACAddressPolicy = "none";
  };

  # The machine's administrators, agents included since they work as the
  # account, may start and stop the shaper without a password
  # (shaper.rules), so an A/B of it against the router alone can turn it
  # on and off around each call without anyone at the keyboard. Wheel
  # could already do it with its password, so this gives it nothing more.
  config.security.polkit.enable = true;
  config.security.polkit.extraConfig = builtins.readFile ./shaper.rules;

  config.systemd.services.shaper = {
    description = "Keep the network's queue here, sorted by the slices";
    # Wanted by nothing, so no boot or switch starts it, until its rebuild
    # on one-way delays passes the bar its review set: five faults in four
    # days came from guessing whose queue a round trip shows and learning
    # rates from its own cuts, and on 3 November it held a 312 Mbit/s
    # download at 81. A router's own SQM shapes the connection meanwhile.
    # Its VM tests start it themselves.
    wantedBy = [ ];
    # The table must be there before the shaper fills its sets, and its
    # queues must be on a link before the first connection leaves by it.
    # The one-way controller reads what the recorder writes, so it wants
    # the recorder running; a recorder that stops leaves the link unlimited.
    wants = [ "network-pre.target" ] ++ lib.optional oneWay "shaper-record.service";
    before = [ "network-pre.target" ];
    after = [ "nftables.service" ];
    requires = [ "nftables.service" ];
    # While a-call-is-on says a call is on, an app's UDP goes as video
    # (shaper.nft says why).
    path = [
      callIsOn
      pkgs.iproute2
      pkgs.iputils
      pkgs.nftables
    ];
    serviceConfig = {
      ExecStart =
        "${shaper}/bin/shaper run"
        + lib.optionalString (vpns != [ ]) " --vpns ${lib.concatStringsSep "," vpns}"
        + lib.optionalString oneWay " --controller one-way --record /var/lib/shaper-record";
      ExecStopPost = "${shaper}/bin/shaper stop";
      Restart = "always";
      RestartSec = 2;
      StateDirectory = "shaper";
      RuntimeDirectory = "shaper";
      RuntimeDirectoryMode = "0755";
      # It reads, and only reads, the file the harness keeps in an
      # account's runtime folder while a pi session runs a turn a person
      # typed, so that one goes ahead of the turns nobody typed. That
      # folder is the account's alone, so reading it takes searching past
      # its mode, and it's the only part of the accounts' folders shown.
      CapabilityBoundingSet = [
        "CAP_NET_ADMIN"
        "CAP_NET_RAW"
        "CAP_DAC_READ_SEARCH"
      ];
      NoNewPrivileges = true;
      ProtectSystem = "strict";
      ProtectHome = "tmpfs";
      BindReadOnlyPaths = [ "-/run/user" ];
      PrivateTmp = true;
    };
  };

  # The shaper's next signal, recorded while the shaper stays off
  # (src/record.rs): each direction's own delay from ICMP timestamps, the
  # gateway's round trip and what the link carried, a file a day for eight
  # days, so a new controller is replayed over a real week before it shapes
  # anything. It needs a raw socket and nothing else. It runs as a user of
  # its own rather than a dynamic one, whose state systemd keeps in a folder
  # only root can open: the days are replayed by the machine's user, and they
  # hold only delays to public resolvers and what the link carried.
  config.users.users.shaper-record = {
    isSystemUser = true;
    group = "shaper-record";
  };
  config.users.groups.shaper-record = { };

  config.systemd.services.shaper-record = {
    description = "Record each direction's delay, for the shaper's rebuild";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" ];
    path = [ pkgs.iproute2 ];
    serviceConfig = {
      ExecStart = "${shaper}/bin/shaper record --state /var/lib/shaper-record";
      Restart = "always";
      RestartSec = 5;
      User = "shaper-record";
      Group = "shaper-record";
      StateDirectory = "shaper-record";
      StateDirectoryMode = "0755";
      AmbientCapabilities = [ "CAP_NET_RAW" ];
      CapabilityBoundingSet = [ "CAP_NET_RAW" ];
      RestrictAddressFamilies = [
        "AF_INET"
        "AF_NETLINK"
        "AF_UNIX"
      ];
      NoNewPrivileges = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      PrivateTmp = true;
    };
  };

  # How each local day's connection did under load, from the recorder's
  # files, for whoever holds the network to a bar (src/day.rs). It runs as
  # each account, since the day file is the account's, and reads the
  # system journal for when the shaper ran.
  config.systemd.user.services.shaper-day = {
    description = "Write how today's connection did under load";
    path = [ pkgs.systemd ];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${shaper}/bin/shaper day";
      Slice = "background.slice";
    };
  };
  config.systemd.user.timers.shaper-day = {
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnCalendar = "hourly";
      Persistent = true;
      RandomizedDelaySec = 300;
    };
  };

  # `shaper status` reads what it holds each link to, and what each tin
  # carried; `shaper proof` measures whether the shaper beats the router.
  config.environment.systemPackages = [ shaperCommand ];
}

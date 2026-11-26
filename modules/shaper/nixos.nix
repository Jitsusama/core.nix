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
# shaper finds it from each direction's own delay, as shaper-record
# measures it with ICMP timestamps, afresh in each stay on a network. A
# network whose line is declared in jitsusama.shaper.networks, by its
# router's hardware address, starts at its hold, and any other at 10 Mbit/s
# up, until the controller's first second leaves it
# unlimited, as it stays until a queue stands on it.
#
# CAKE's four tins follow the slices: voice for names, time and the
# shaper's own pings; video for Tuple's daemon, an app's UDP other than
# 443, which is how a browser's call sends, and what an app sends Meet's
# media servers, which take a call on 443; bulk for the jobs, background
# and containers slices, the account's user.slice, Nix's builds, virtual
# machines and whatever this machine forwards for them; best effort for the
# rest, Spotify, Chrome, Slack, pi's own requests and what Joel waits on at
# a terminal among them. What a link reaches without the router isn't
# slowed.
#
# A VPN leaves and arrives by the connection as one flow, whatever it
# carries, so CAKE there can't tell a name lookup inside it from a build's
# upload, or a model's reply from a git fetch. A VPN named in
# jitsusama.shaper.vpns gets a CAKE of its own each way, sorted by the same
# classes, so its queues form inside it.
#
# The upload is shared between lanes (src/lanes.rs): Joel's traffic
# straight on the connection, the VPN's, and agents' bulk straight on the
# connection, each a class of the connection's HTB at the controller's
# rate. Joel comes first on both lanes; while agents have something
# waiting, a quarter of the rate is theirs, split evenly between the lanes
# on which they wait, and they take whatever Joel leaves.
#
# Each time the shaper starts, the kernel warns "htb: cake qdisc 2: is
# non-work-conserving?" for the IFB, and never again until the queues are
# made afresh. It's expected: CAKE keeps its own rate on the download, so
# when HTB asks it for a packet it holds back, it gives none, and HTB says
# so the first time any child does (sch_htb.c, qdisc_warn_nonwc in
# pkt_sched.h).
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
  # playing, and reads the default route with ip; its uploads read zeros
  # with head and wait with sleep; systemd-run and systemctl are the
  # system's own. What else it waits on, such as the calendar, the
  # machine names in proofWaitsWhile.
  waitsWhile = config.jitsusama.shaper.proofWaitsWhile;
  waitsFlag = lib.optionalString (waitsWhile != [ ]) (
    "--set-default SHAPER_PROOF_WAITS_WHILE " + lib.escapeShellArg (builtins.toJSON waitsWhile)
  );
  proofTools = [
    pkgs.coreutils
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
  networks = config.jitsusama.shaper.networks;
  networksFile = pkgs.writeText "shaper-networks.json" (builtins.toJSON networks);
  # The port src/house.rs hears another machine's reports on.
  housePort = 47110;
  houses = lib.filter (house: house != null) (lib.catAttrs "house" (lib.attrValues networks));
  # An address as the firewall and src/networks.rs read it alike.
  address = import ./address.nix lib;
  # A house's addresses, one written alone taken as a list of one, so a
  # machine that named one before names it the same way still. None is
  # allowed only beside a key, which alone then says who sent a report.
  addresses = lib.types.coercedTo address lib.toList (lib.types.listOf address);
  # Houses heard by their key alone, from any private IPv4 address.
  # Each holding the key a report was signed under hears it, so one
  # machine may be the house on several networks (house any round 2's
  # F12 and F15).
  unaddressed = lib.filter (house: house.from == [ ]) houses;
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
    type = lib.types.nullOr (lib.types.enum [ "one-way" ]);
    default = null;
    description = ''
      What sets the shaper's rates, and whether it starts at boot: `one-way`
      (src/oneway.rs), from each direction's own delay as shaper-record
      measures it, the controller `shaper replay` scores, and the only one
      left. Left null, the shaper is installed and nothing starts it, as a
      machine that never named the one-way controller had it before; the
      recorder runs either way, so a machine's week can be replayed before
      it shapes. Setting
      it back to null takes the start at boot away but leaves a running
      shaper shaping until `systemctl stop shaper` or the next boot. The
      round trip's, from pings to distant resolvers, went on 18 November.
    '';
  };

  options.jitsusama.shaper.networks = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options.up = lib.mkOption {
          type = lib.types.nullOr lib.types.ints.positive;
          default = null;
          example = 12000;
          description = ''
            What the network's line carries up, in kbit/s, or null where
            it isn't known, which leaves the upload unguarded there.
          '';
        };
        options.down = lib.mkOption {
          # Under 6 Mb/s a value is most likely written in Mb/s, and a
          # call's room under one that low would fall to the download's
          # floor.
          type = lib.types.nullOr (lib.types.ints.between 6000 10000000);
          default = null;
          example = 90000;
          description = ''
            What the network's line carries down, in kbit/s, or null where
            it isn't known. While a call is on and the download carries
            nine tenths of it, or the controller holds it under a full
            line, having cut it in the last few seconds or with the
            download filling its hold, the download is held at nine
            tenths, under the router's queue, so
            it queues in this machine's CAKE, which puts the call first,
            rather than in a router's flow queues, where a call that
            shares one with a bulk download loses what it loses. The
            controller may hold it lower, and the hold goes when the call
            ends. Declare the line at its slowest busy hour, and again
            when it's made faster: held, the download can't show a faster
            line, so each call would hold it at nine tenths of the old one.
          '';
        };
        options.links = lib.mkOption {
          type = lib.types.attrsOf (
            lib.types.submodule {
              options.down = lib.mkOption {
                # As the network's own, and for the same reasons.
                type = lib.types.nullOr (lib.types.ints.between 6000 10000000);
                default = null;
                example = 20000;
                description = ''
                  What this link reaches the network's line at, down, in
                  kbit/s, or null to take the network's `down`. While a
                  call is on, the link's download is held under it as
                  under `down`, and the hold goes when the call ends.
                '';
              };
            }
          );
          default = { };
          example = {
            wlan0.down = 20000;
          };
          description = ''
            The network's links that reach its line slower than the line
            carries, by interface name, each with its own `down`: Wi-Fi
            through an access point whose own queue holds a call behind
            a download, while a cable to the same router needs no hold.
            A link not named takes the network's `down`.
          '';
        };
        options.most = lib.mkOption {
          # Under 4 Mb/s a value is most likely written in Mb/s, and one
          # that low starves agents' lane, as the shaper's trials refuse.
          type = lib.types.nullOr (lib.types.ints.between 4000 10000000);
          default = null;
          example = 9500;
          description = ''
            The most the upload is held to on this network, in kbit/s,
            probes included, or null for no ceiling. Set it under a router
            that shapes the upload itself, a little under that router's
            rate, so the queue stands here, where Joel's traffic goes
            first, rather than in the router's. A trial may hold it lower,
            never higher. Under nine tenths of `up`, it also stands for
            the line's hold below: the router's queue is looked for, and
            the guard returns the upload, against it rather than `up`.
            With `up` null it holds the upload all the same, and guards
            nothing.
          '';
        };
        options.house = lib.mkOption {
          type = lib.types.nullOr (
            lib.types.submodule {
              options.most = lib.mkOption {
                type = lib.types.ints.between 4000 10000000;
                example = 7000;
                description = ''
                  The most this machine and the other send up between
                  them, in kbit/s, under the rate where the router's queue
                  starts.
                '';
              };
              options.from = lib.mkOption {
                type = addresses;
                default = [ ];
                example = [
                  "192.0.2.7"
                  "192.0.2.8"
                ];
                description = ''
                  The addresses the other machine reports from, one or a
                  list, the only ones heard, so a report from anywhere else
                  holds nothing. Name one for each interface it may send
                  by, its Wi-Fi's and its cable's, since it reports by the
                  one its routes send by: what it sent is read from the
                  fresh report saying the most, so one from an address it
                  isn't sending by can only tighten the hold. The address
                  alone says a report is the other machine's, so any
                  device on the line could forge one by sending from it: for
                  as long as it keeps sending, it holds this machine's
                  upload as low as 4 Mb/s, as a trial may, or lets it go.
                  Name a private IPv4 or unique local IPv6 address, not a
                  link-local one: every link the machine has, a virtual
                  machine's bridge among them, shares a link-local address,
                  and the rule names no interface. An IPv4 address written
                  as IPv6 is refused. Make it a reserved address the other
                  machine sends from: reports from a new lease, or from a
                  temporary IPv6 address, are dropped without a word, and
                  the house holds nothing. house-report sends over IPv4
                  alone, for that reason. With a `keyFile`, a report from
                  one of these is taken only when it's signed. Left empty,
                  which needs a `keyFile`, a signed report is taken from
                  any address, so the house is heard wherever a lease or
                  a cable moves the other machine; the port is then open
                  to every private IPv4 address, on every link, so the
                  other machine must report over IPv4, as house-report
                  does, and the key and its stamp alone keep a device that
                  reaches it from holding this machine's upload.
                '';
              };
              options.keyFile = lib.mkOption {
                type = lib.types.nullOr (
                  lib.types.pathWith {
                    inStore = false;
                    absolute = true;
                  }
                );
                default = null;
                example = "/var/lib/shaper/house.key";
                description = ''
                  A file holding the house's key, at least 16 bytes, which
                  the other machine's house-report signs each report with
                  (its `keyFile`), so a device on the line can't forge one
                  from the other machine's address. Set, only signed
                  reports are taken, stamped within 3 s of this machine's
                  clock; while the file can't be read, the house holds
                  nothing and the journal says why. Unset, a report is
                  taken by its address alone. The path is read by the
                  shaper, which runs as root, so make it root's alone,
                  as a root shell does with `install -m 0400 /dev/null
                  FILE`, then writes the reporter's key into it, or
                  makes the key with `head -c 32 /dev/urandom > FILE`
                  and writes the same bytes into the reporter's, through
                  a pipe rather than a file another account could read:
                  any account that can read it may sign a report. A key
                  either machine ever let another account read is
                  replaced on both by new bytes, made once that way,
                  rather than given a new mode or copied back in, since
                  a copy taken of it stays good. Each machine's file is
                  made anew by that `install` before the bytes go in,
                  since a write into a file already there keeps its
                  mode. The shaper's service sees no /root, /home,
                  /tmp or /var/tmp, so keep it elsewhere, as under
                  /var/lib, spelled plainly and not through a link;
                  houses naming one address name one key. Never a Nix
                  path or a value in the store.
                '';
              };
            }
          );
          default = null;
          description = ''
            Another machine on this network's line, which says each second
            what it sends up, to UDP port ${toString housePort}. While it
            has said so in the last 3 s, this machine's upload is held under
            `most` less what it sent, never under 4 Mb/s, where a call
            still fits; with no word lately, the house holds nothing. Its uploads
            fill the router's queue this machine's VPN waits in, which this
            machine never sees otherwise.
          '';
        };
      }
    );
    default = { };
    example = {
      "00:00:5e:00:53:01".up = 12000;
    };
    description = ''
      What each network's line is known to carry, keyed by its router's
      hardware address in lower case. On a network that declares its `up`
      the one-way controller keeps the upload near the line: a queue past
      its CAKE counts as the router's only while the link carries near the
      line, and an upload held under four fifths of it for a minute, with
      no cut that drained a queue, goes back to nine tenths. Leaving a
      network out, or its `up` null, turns both off on it; its `down`
      gives a call room on the download.
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

  # A house's addresses alone reach the port its reports go to. The rule
  # holds on every network, but a report holds only the links of the
  # network that names the house. A keyed house naming no address is
  # heard from any private IPv4 address, house-report sending over IPv4
  # alone, on every link, since the machine moves between them; its key
  # alone says a report is its own (house any round 1's F3, F11, F16 and
  # F18).
  config.networking.firewall.extraInputRules =
    lib.concatMapStrings (address: ''
      ${
        if lib.hasInfix ":" address then "ip6" else "ip"
      } saddr ${address} udp dport ${toString housePort} accept
    '') (lib.unique (lib.concatMap (house: house.from) houses))
    + lib.optionalString (unaddressed != [ ]) ''
      ip saddr { 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16 } udp dport ${toString housePort} accept
    '';

  # The shaper's service sees an empty /root, /home, /tmp and /var/tmp, so
  # a key kept there would read as missing, as would one spelled through
  # those by `//`, `/./` or `/../`; and the ear holds each address to one
  # key, so two houses naming one address, spelled alike, must name the
  # same key. A link into those folders still passes, for the journal to
  # say.
  config.assertions =
    let
      keyed = lib.filter (house: house.keyFile != null) houses;
      unseen = [
        "/root/"
        "/home/"
        "/tmp/"
        "/var/tmp/"
      ];
      unplain = [
        "//"
        "/./"
        "/../"
      ];
      hides =
        path:
        lib.any (dir: lib.hasPrefix dir path) unseen
        || lib.any (part: lib.hasInfix part path) unplain
        || lib.hasSuffix "/." path
        || lib.hasSuffix "/.." path;
      hidden = lib.filter hides (map (house: toString house.keyFile) keyed);
      naming = address: lib.filter (house: lib.elem address house.from) keyed;
      keysOf = address: lib.unique (map (house: house.keyFile) (naming address));
      shared = lib.filter (address: lib.length (keysOf address) > 1) (
        lib.unique (lib.concatMap (house: house.from) keyed)
      );
    in
    [
      {
        assertion = hidden == [ ];
        message =
          "jitsusama.shaper.networks: a house's keyFile can't be under /root, /home, "
          + "/tmp or /var/tmp, which the shaper's service can't see, nor spelled with "
          + "//, /./ or /../: "
          + lib.concatStringsSep ", " hidden;
      }
      {
        assertion = lib.all (house: house.keyFile != null) unaddressed;
        message =
          "jitsusama.shaper.networks: a house naming no address in `from` must name "
          + "a keyFile, since its key alone says a report is the other machine's";
      }
      {
        assertion = shared == [ ];
        message =
          "jitsusama.shaper.networks: houses naming one address must name one "
          + "keyFile: "
          + lib.concatStringsSep ", " shared;
      }
    ];

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

  # Where the same administrators may write a trial of how the upload
  # finds its line (src/live.rs), for an A/B of it on the real line
  # without a switch. The shaper holds each value within bounds that keep
  # a standing queue under what a call hears, and stopping the shaper
  # would do more. A trial's ceiling may hold the upload lower without a
  # password, but never under 4 Mb/s, where a call still fits, and only
  # for ten minutes after each write.
  config.systemd.tmpfiles.rules = [ "d /run/shaper-trial 0775 root wheel -" ];

  config.systemd.services.shaper = {
    description = "Keep the network's queue here, sorted by the slices";
    # A machine that names the controller has it at boot: a day of the
    # one-way controller on against off on a real link, eighteen pairs
    # overnight, found it as good or better on every bar, and a boot that
    # left it off left agents' uploads filling the router's queue until
    # someone started it. One that never named it isn't shaped by an
    # update. A switch never starts a shaper someone stopped, so
    # `systemctl stop shaper` stays the way to turn it off. The round
    # trip's controller, which set rates from pings, is gone: five faults
    # in four days came from guessing whose queue a round trip shows, and
    # on 3 November it held a 312 Mbit/s download at 81.
    wantedBy = lib.optional (config.jitsusama.shaper.controller != null) "multi-user.target";
    # The table must be there before the shaper fills its sets, and its
    # queues must be on a link before the first connection leaves by it.
    # The controller reads what the recorder writes, so it wants the
    # recorder running; a recorder that stops leaves the link unlimited.
    wants = [
      "network-pre.target"
      "shaper-record.service"
    ];
    before = [ "network-pre.target" ];
    after = [ "nftables.service" ];
    requires = [ "nftables.service" ];
    # While a-call-is-on says a call is on, an app's UDP goes as video
    # (shaper.nft says why).
    path = [
      callIsOn
      config.systemd.package
      pkgs.iproute2
      pkgs.iputils
      pkgs.nftables
    ];
    # A switch restarts it rather than stopping the old and starting the
    # new, and a restart's stop leaves the queues for the new shaper to
    # take back (src/restart.rs): at 13:29 on 16 November the link went
    # 5 s unshaped, and the gateway's round trip rose from 10 to 30 ms to
    # 102. A stop, or a restart after a crash, still takes them all away.
    stopIfChanged = false;
    serviceConfig = {
      ExecStart =
        "${shaper}/bin/shaper run"
        + lib.optionalString (vpns != [ ]) " --vpns ${lib.concatStringsSep "," vpns}"
        + lib.optionalString (networks != { }) " --networks ${networksFile}";
      ExecStopPost = "${shaper}/bin/shaper stop --unless-restarting shaper.service";
      # The stop's SIGTERM goes to the shaper alone, which ends its loop
      # itself. Sent to the whole unit, it also killed a tc the loop was
      # reading, whose empty answer read as a queue gone, so the loop put
      # the VPN's turn on again and matchall refused it: "File exists" in
      # the restart test, twice on 21 November. A failed read no longer
      # reads as one (src/main.rs), and a command let finish still beats
      # one cut partway. What is left once the shaper exits is still
      # killed.
      KillMode = "mixed";
      # A stop waits on whatever command the loop is in, and a stuck one
      # would hold a switch for systemd's 90 s. Stops took 0.04 to 3.6 s on
      # 21 November, the slowest under a load of 160. It bounds the stop's
      # sweep too, one tc read for each of a handful of devices, which took
      # 46 ms. Past this the shaper is killed. The stop's sweep names in the
      # journal any device it can't read, and one cut off at this bound
      # leaves those it hadn't reached at their last rate, until a shaper
      # starts and takes them back.
      TimeoutStopSec = 15;
      Restart = "always";
      RestartSec = 2;
      StateDirectory = "shaper";
      RuntimeDirectory = "shaper";
      RuntimeDirectoryMode = "0755";
      # It reads, and only reads, the file the harness keeps in an
      # account's runtime folder while a pi session runs a turn a person
      # typed, so that one goes ahead of the turns nobody typed, and the
      # names in its shaper/first, the work put ahead of the rest. That
      # folder is the account's alone, so reading it takes searching past
      # its mode, and it's the only part of the accounts' folders shown.
      # What's there is the account's, so a link isn't followed, at most
      # 16 names are read, and a name nft could misread is left out.
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

  # What the shaper sets its rates from (src/record.rs): each direction's
  # own delay from ICMP timestamps, the gateway's round trip and what the
  # link carried, a file a day for eight days, so a change to the
  # controller is replayed over a real week before it shapes anything. It
  # needs a raw socket and nothing else. It runs as a user of its own rather
  # than a dynamic one, whose state systemd keeps in a folder only root can
  # open: the days are replayed by the machine's user, and they
  # hold only delays to public resolvers and what the link carried.
  config.users.users.shaper-record = {
    isSystemUser = true;
    group = "shaper-record";
  };
  config.users.groups.shaper-record = { };

  config.systemd.services.shaper-record = {
    description = "Record each direction's delay, for the shaper's controller and replays";
    wantedBy = [ "multi-user.target" ];
    after = [ "network.target" ];
    path = [ pkgs.iproute2 ];
    # A switch restarts it in its start phase, beside the shaper, rather
    # than stopping it with the units it stops first: on 16 November at
    # 20:18 the record went blind from 20:18:46 to 20:18:53, and the
    # shaper's restart, the very moment worth seeing, fell inside.
    stopIfChanged = false;
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
    # A person's: a system account has no home to keep the day in.
    unitConfig.ConditionUser = "!@system";
    path = [ pkgs.systemd ];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${shaper}/bin/shaper day";
      Slice = "background.slice";
      # journalctl's reads of the journal's files are charged to the unit as
      # page cache: 350 MB cold on 13 November, though the day itself takes
      # 6 MB. That cache is clean, so held to this the kernel takes it back
      # as the reads go, for a second more of running.
      MemoryHigh = "64M";
    };
  };
  config.systemd.user.timers.shaper-day = {
    wantedBy = [ "timers.target" ];
    unitConfig.ConditionUser = "!@system";
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

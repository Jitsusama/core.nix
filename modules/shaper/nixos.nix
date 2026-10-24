# The network in the same order as the CPU, memory and disk: a call, music
# and what Joel waits on before the work he started, both ways.
#
# The queue that fills when this machine sends isn't its own: it's the
# modem's or the router's, first come first served, so a call's packets
# wait there behind a build's upload. With a 1 GB download running, pings
# to 1.1.1.1 over Ethernet took 173 ms at the median and 288 ms at the 95th
# percentile, against 17 ms quiet; with an upload, 197 ms through WARP. The
# line carried about 11 Mbit/s up. So on every link with a default route the
# shaper sends and receives a little slower than the line can take, through
# CAKE, and the queue forms here instead, short and sorted. How fast that is
# differs on every network, and on Wi-Fi from minute to minute, so the
# shaper finds it from what pings to four distant resolvers do, cutting
# the moment they slow and climbing while they don't, and keeps what each
# network held, by its router's hardware address. A network never met
# starts at 10 Mbit/s up and 50 down and climbs within seconds.
#
# CAKE's four tins follow the slices: voice for names, time and the
# shaper's own pings; video for Tuple's daemon and an app's UDP other than
# 443, which is how a browser's call sends; bulk for the jobs, background
# and containers slices, the account's user.slice, Nix's builds, virtual
# machines and whatever this machine forwards for them; best effort for the
# rest, Spotify, Chrome, Slack, pi's own requests and what Joel waits on at
# a terminal among them. Bulk gets
# the whole line while nothing else wants it, and a sixteenth while
# something does. What a link reaches without the router isn't slowed.
#
# A tunnel such as a VPN's leaves by the uplink as one flow, whatever it
# carries, so CAKE there can't tell a name lookup inside it from a build's
# upload. A tunnel named in jitsusama.shaper.tunnels gets a CAKE of its
# own, sorted by the same classes, at a little under the uplink's rate, so
# its queue forms inside it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  shaper = pkgs.callPackage ./package.nix { };
  tunnels = config.jitsusama.shaper.tunnels;
in
{
  imports = [ ../nftables/nixos.nix ];

  options.jitsusama.shaper.tunnels = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [ "wg0" ];
    description = ''
      The tunnels to shape inside, by their links' names: those that ride
      an uplink as one flow and carry traffic this machine sends.
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

  config.systemd.services.shaper = {
    description = "Keep the network's queue here, sorted by the slices";
    wantedBy = [ "multi-user.target" ];
    # The table must be there before the shaper fills its sets, and its
    # queues must be on a link before the first connection leaves by it.
    wants = [ "network-pre.target" ];
    before = [ "network-pre.target" ];
    after = [ "nftables.service" ];
    requires = [ "nftables.service" ];
    path = [
      pkgs.iproute2
      pkgs.iputils
      pkgs.nftables
    ];
    serviceConfig = {
      ExecStart =
        "${shaper}/bin/shaper run"
        + lib.optionalString (tunnels != [ ]) " --tunnels ${lib.concatStringsSep "," tunnels}";
      ExecStopPost = "${shaper}/bin/shaper stop";
      Restart = "always";
      RestartSec = 2;
      StateDirectory = "shaper";
      RuntimeDirectory = "shaper";
      RuntimeDirectoryMode = "0755";
      CapabilityBoundingSet = [
        "CAP_NET_ADMIN"
        "CAP_NET_RAW"
      ];
      NoNewPrivileges = true;
      ProtectSystem = "strict";
      ProtectHome = true;
      PrivateTmp = true;
    };
  };

  # `shaper status` reads what it holds each link to, and what each tin
  # carried.
  config.environment.systemPackages = [ shaper ];
}

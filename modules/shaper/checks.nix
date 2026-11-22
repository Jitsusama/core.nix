# The checks of shaper, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
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
  services = (bareNixos [ self.nixosModules.shaper ]).config.systemd.services;
  named =
    (bareNixos [
      self.nixosModules.shaper
      { jitsusama.shaper.controller = "one-way"; }
    ]).config.systemd.services;
  # A machine whose two networks each name a house, one reporting over
  # IPv4 and one over IPv6, and one naming an IPv4 address written as
  # IPv6, which the firewall would match only as IPv6.
  housed =
    from:
    (bareNixos [
      self.nixosModules.shaper
      {
        jitsusama.shaper.networks = lib.mapAttrs (_: address: {
          up = 12000;
          house = {
            most = 7000;
            from = address;
          };
        }) from;
      }
    ]).config.networking.firewall.extraInputRules;
  rules = housed {
    "00:00:5e:00:53:01" = "192.0.2.7";
    "00:00:5e:00:53:02" = "2001:db8::7";
  };
  # The port src/house.rs listens on, read from it so the two can't part.
  port = lib.pipe ./src/house.rs [
    builtins.readFile
    (lib.splitString "\n")
    (map (builtins.match "pub const PORT: u16 = ([0-9_]+);"))
    (lib.findFirst (found: found != null) [ "none" ])
    lib.head
    (lib.replaceStrings [ "_" ] [ "" ])
  ];
  opened = lib.all (rule: lib.hasInfix rule rules) [
    "ip saddr 192.0.2.7 udp dport ${port} accept"
    "ip6 saddr 2001:db8::7 udp dport ${port} accept"
  ];
  mapped =
    (builtins.tryEval (housed {
      "00:00:5e:00:53:01" = "::ffff:c000:207";
    })).success;
  unasked = services.shaper.wantedBy == [ ];
  asked = builtins.elem "multi-user.target" named.shaper.wantedBy;
  recorded = builtins.elem "multi-user.target" services.shaper-record.wantedBy;
in
# What follows needs Linux to build or to run, so a Mac leaves it out.
onLinux {
  # The shaper's tests run as it builds: each set, route and queue is read
  # from what the kernel prints, every link to one router counts on its
  # line, a recorded week replays through a controller and is scored, and
  # the one-way controller sets each direction from its own delay.
  shaper-finds-the-rate = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in the shaper, its tests included.
  shaper-passes-clippy = passesClippy "shaper-passes-clippy" (pkgs.callPackage ./package.nix { });

  # A switch restarts the shaper and its recorder in place, in its start
  # phase, rather than stopping them before it activates. The shaper's
  # restart keeps its queues (src/restart.rs); the recorder's keeps the
  # record on the shaper's restart, which on 16 November it was blind to
  # from 20:18:46 to 20:18:53 and from 21:51:52 to 21:52:02.
  shaper-stays-through-a-switch =
    holds "shaper-stays-through-a-switch"
      "A switch would stop the shaper or its recorder first, so its queues or its record go."
      (
        services.shaper.restartIfChanged
        && !services.shaper.stopIfChanged
        && services.shaper-record.restartIfChanged
        && !services.shaper-record.stopIfChanged
      );

  # A network's house opens the port the shaper hears its reports on to
  # the house's address alone, over IPv4 or IPv6 as it is written, and an
  # IPv4 address written as IPv6, which would open the port to no report,
  # is refused.
  shaper-hears-only-the-house = holds "shaper-hears-only-the-house" (
    if !opened then
      "A house doesn't open UDP port ${port} to its address alone: ${rules}"
    else
      "An IPv4 address written as IPv6 was taken as a house's."
  ) (opened && !mapped);

  # Only a machine that names the controller shapes from boot: every
  # graphical machine imports the module, and one that never asked for
  # the one-way controller was never shaped before it was the only one.
  # Its recorder starts either way, so its week can be replayed first.
  shaper-starts-when-asked = holds "shaper-starts-when-asked" (
    if !unasked then
      "The shaper would start at boot on a machine that never named its controller."
    else if !asked then
      "The shaper wouldn't start at boot on a machine that named its controller."
    else
      "The recorder wouldn't start at boot on a machine that never named the controller."
  ) (unasked && asked && recorded);
}
